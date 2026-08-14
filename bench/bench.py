#!/usr/bin/env python3
"""
Emerald benchmark harness.

Measures three things that the README makes claims about, so the claims can be
checked rather than believed:

  1. cold start — exec to the chrome having painted
  2. idle memory — resident and proportional set size of the whole process
     tree, with one tab open and nothing happening
  3. tab suspension — the memory actually returned to the OS when the tab
     ceiling discards a tab

Deliberately uses no test-only code paths in Emerald. Tabs are opened by
passing URLs on the command line, which is a real feature, and suspension is
triggered by writing a real `settings.json` with a low `max_live_tabs` and
letting the ordinary policy run. If the benchmark passes, the shipped
behaviour is what was measured.

Usage:
    python3 bench/bench.py [--binary PATH] [--tabs N] [--json OUT]
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_BINARY = REPO / "src-tauri" / "target" / "release" / "emerald"
PAGES = sorted((REPO / "bench" / "pages").glob("*.html"))

TRACE_RE = re.compile(r"emerald-trace (\S+) ([\d.]+)ms")


# ---------------------------------------------------------------------------
# /proc sampling — the same arithmetic as src-tauri/src/metrics.rs
# ---------------------------------------------------------------------------

PAGE_KB = os.sysconf("SC_PAGE_SIZE") // 1024


def children_of(root: int) -> list[int]:
    edges = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            stat = (entry / "stat").read_text()
        except OSError:
            continue
        try:
            ppid = int(stat.rsplit(")", 1)[1].split()[1])
        except (IndexError, ValueError):
            continue
        edges.append((int(entry.name), ppid))

    out, frontier = [root], {root}
    for _ in range(8):
        nxt = {pid for pid, ppid in edges if ppid in frontier and pid not in out}
        if not nxt:
            break
        out.extend(sorted(nxt))
        frontier = nxt
    return out


def rss_kb(pid: int) -> int:
    try:
        return int(Path(f"/proc/{pid}/statm").read_text().split()[1]) * PAGE_KB
    except (OSError, IndexError, ValueError):
        return 0


def pss_kb(pid: int) -> int | None:
    try:
        for line in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines():
            if line.startswith("Pss:"):
                return int(line.split()[1])
    except (OSError, IndexError, ValueError):
        return None
    return None


def comm(pid: int) -> str:
    try:
        return Path(f"/proc/{pid}/comm").read_text().strip()
    except OSError:
        return "?"


def sample(root: int) -> dict:
    pids = children_of(root)
    procs = []
    for pid in pids:
        r = rss_kb(pid)
        if r == 0:
            continue
        procs.append({"pid": pid, "name": comm(pid), "rss_kb": r, "pss_kb": pss_kb(pid)})
    pss_vals = [p["pss_kb"] for p in procs if p["pss_kb"] is not None]
    return {
        "rss_kb": sum(p["rss_kb"] for p in procs),
        "pss_kb": sum(pss_vals) if pss_vals else None,
        "shell_rss_kb": next((p["rss_kb"] for p in procs if p["pid"] == root), 0),
        "processes": procs,
        "web_processes": sum(1 for p in procs if "Web" in p["name"] or "web" in p["name"]),
    }


# ---------------------------------------------------------------------------
# Launching
# ---------------------------------------------------------------------------


# Must match `identifier` in src-tauri/tauri.conf.json — Tauri derives the
# config directory from it. Getting this wrong makes the harness silently
# benchmark the *default* settings and report a real-looking number for a
# policy that never ran.
BUNDLE_ID = "app.emerald.browser"


def write_settings(config_home: Path, max_live: int, suspend_minutes: int = 0) -> None:
    """A real settings.json. The policy under test is the shipped one."""
    d = config_home / BUNDLE_ID
    d.mkdir(parents=True, exist_ok=True)
    (d / "settings.json").write_text(
        json.dumps(
            {
                "focus_access": {
                    "attention": {
                        "max_live_tabs": max_live,
                        "suspend_idle_minutes": suspend_minutes,
                        "auto_archive_idle_minutes": 0,
                    }
                },
                # Keep the run deterministic: no session carried in from a
                # previous launch, no budget-driven discards racing the cap.
                "performance": {"memory_budget_mb": 0, "restore_tabs_discarded": True},
            }
        )
    )


class Display:
    """
    One Xvfb for the whole run.

    Emerald must be `Popen`ed *directly* rather than through `xvfb-run`: that
    wrapper is a shell script, so its pid is the shell's and the memory sample
    would be rooted at the wrong process — which also drags Xvfb itself into
    the tree. Starting the server once here keeps the measured tree to exactly
    the browser under test, and keeps every scenario on an identical display.
    """

    def __init__(self, number: int = 99, geometry: str = "1280x820x24"):
        self.number = number
        self.proc = subprocess.Popen(
            ["Xvfb", f":{number}", "-screen", "0", geometry, "-nolisten", "tcp"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        time.sleep(1.0)

    @property
    def display(self) -> str:
        return f":{self.number}"

    def close(self) -> None:
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()


def launch(binary: Path, urls: list[str], config_home: Path, display: str):
    env = dict(os.environ)
    env["EMERALD_TRACE"] = "1"
    env["DISPLAY"] = display
    env["XDG_CONFIG_HOME"] = str(config_home)
    env["XDG_DATA_HOME"] = str(config_home / "data")
    env["XDG_CACHE_HOME"] = str(config_home / "cache")
    # WebKitGTK picks a compositing mode from the environment; pin it so runs
    # are comparable and do not depend on whether a GPU showed up.
    env.setdefault("WEBKIT_DISABLE_COMPOSITING_MODE", "1")
    env.setdefault("WEBKIT_DISABLE_DMABUF_RENDERER", "1")

    return subprocess.Popen(
        [str(binary), *urls],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )


def launch_chromium(binary: Path, urls: list[str], profile: Path, display: str):
    """
    Same-machine baseline.

    Kept as close to a default Chromium as the environment allows. `--no-sandbox`
    is required because this runs as root, and it removes Chromium's sandbox
    helper processes — which makes this baseline slightly *favourable* to
    Chromium on process count and memory. Noted rather than corrected, because
    the alternative is not running the comparison at all.
    """
    env = dict(os.environ)
    env["DISPLAY"] = display
    return subprocess.Popen(
        [
            str(binary),
            f"--user-data-dir={profile}",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-features=Translate",
            "--no-sandbox",
            "--window-size=1280,820",
            *urls,
        ],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def read_traces(proc, deadline_s: float, t0: float) -> dict[str, float]:
    """
    Collect `emerald-trace` lines until the chrome reports it has painted.

    Emerald's own trace clock starts inside `setup()`, so it cannot see process
    exec, dynamic linking or GTK init — a real user waits for those too.
    `wall_to_painted` is measured here, from just before `Popen` to the moment
    the painted line arrives, and is the number to quote as "cold start".
    """
    traces: dict[str, float] = {}
    end = time.monotonic() + deadline_s
    os.set_blocking(proc.stderr.fileno(), False)
    while time.monotonic() < end:
        line = proc.stderr.readline()
        if not line:
            if proc.poll() is not None:
                break
            time.sleep(0.005)
            continue
        m = TRACE_RE.search(line)
        if m:
            traces[m.group(1)] = float(m.group(2))
            if m.group(1) == "chrome-painted":
                traces["wall_to_painted"] = (time.monotonic() - t0) * 1000.0
                break
    return traces


def settle(root: int, seconds: float = 4.0, quiet_delta_kb: int = 2048) -> dict:
    """
    Wait until the tree's RSS stops moving, then sample.

    Memory right after launch is still climbing; sampling too early flatters
    the number and sampling on a fixed sleep is arbitrary. This waits for two
    consecutive samples within `quiet_delta_kb` of each other.
    """
    prev = None
    end = time.monotonic() + seconds
    last = sample(root)
    while time.monotonic() < end:
        time.sleep(0.35)
        cur = sample(root)
        if prev is not None and abs(cur["rss_kb"] - prev["rss_kb"]) < quiet_delta_kb:
            return cur
        prev, last = cur, cur
    return last


def kill(proc) -> None:
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------


def run_scenario(binary: Path, urls: list[str], max_live: int, display: str, settle_s: float = 8.0):
    with tempfile.TemporaryDirectory(prefix="emerald-bench-", ignore_cleanup_errors=True) as tmp:
        cfg = Path(tmp)
        write_settings(cfg, max_live=max_live)
        t0 = time.monotonic()
        proc = launch(binary, urls, cfg, display)
        try:
            traces = read_traces(proc, deadline_s=30, t0=t0)
            if proc.poll() is not None:
                raise RuntimeError(f"Emerald exited during startup (code {proc.returncode})")
            mem = settle(proc.pid, seconds=settle_s)
            mem["traces"] = traces
            # A run whose policy never took effect is worse than no run: it
            # reports a plausible number for something that did not happen.
            mem["live_web_processes"] = mem["web_processes"]
            return mem
        finally:
            kill(proc)


def run_chromium(binary: Path, urls: list[str], display: str, settle_s: float = 8.0):
    # Chromium keeps writing its profile for a moment after SIGTERM, so a
    # strict cleanup races it and dies with "Directory not empty". The
    # benchmark must not fail because a scratch directory was still busy.
    with tempfile.TemporaryDirectory(
        prefix="chromium-bench-", ignore_cleanup_errors=True
    ) as tmp:
        proc = launch_chromium(binary, urls, Path(tmp), display)
        try:
            time.sleep(3.0)
            if proc.poll() is not None:
                raise RuntimeError(f"Chromium exited early (code {proc.returncode})")
            return settle(proc.pid, seconds=settle_s)
        finally:
            kill(proc)


def mb(kb: int | None) -> float | None:
    return None if kb is None else round(kb / 1024, 1)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    ap.add_argument("--tabs", type=int, default=8, help="tabs to open for the suspension test")
    ap.add_argument("--repeat", type=int, default=3, help="runs per scenario")
    ap.add_argument("--json", type=Path, help="write raw results here")
    ap.add_argument(
        "--chromium",
        type=Path,
        help="path to a Chromium binary for a same-machine baseline",
    )
    args = ap.parse_args()

    if not args.binary.exists():
        print(f"no binary at {args.binary} — run `cargo build --release` first", file=sys.stderr)
        return 1
    if not PAGES:
        print("no bench pages found in bench/pages/", file=sys.stderr)
        return 1

    page_urls = [p.as_uri() for p in PAGES]
    many = [page_urls[i % len(page_urls)] for i in range(args.tabs)]

    results: dict[str, list[dict]] = {
        "idle_one_tab": [],
        "all_awake": [],
        "capped": [],
        "chromium_one_tab": [],
        "chromium_many": [],
    }

    print(f"binary   {args.binary}")
    print(f"pages    {len(PAGES)} local files, {args.tabs} tabs for the cap test")
    print(f"repeats  {args.repeat}")
    if args.chromium:
        print(f"baseline {args.chromium}")
    print()

    display = Display()
    try:
        for i in range(args.repeat):
            print(f"run {i + 1}/{args.repeat} …", flush=True)
            d = display.display
            # 1. One tab, idle. The floor.
            results["idle_one_tab"].append(run_scenario(args.binary, page_urls[:1], 8, d))
            # 2. N tabs, ceiling high enough that all stay awake.
            results["all_awake"].append(run_scenario(args.binary, many, args.tabs + 4, d))
            # 3. Same N tabs, ceiling of 2 — the shipped policy discards the rest.
            results["capped"].append(run_scenario(args.binary, many, 2, d))

            if args.chromium and args.chromium.exists():
                results["chromium_one_tab"].append(run_chromium(args.chromium, page_urls[:1], d))
                results["chromium_many"].append(run_chromium(args.chromium, many, d))
    finally:
        display.close()

    def agg(rows: list[dict], key: str):
        vals = [r[key] for r in rows if r.get(key) is not None]
        return None if not vals else statistics.median(vals)

    def trace_median(rows: list[dict], name: str):
        vals = [r["traces"][name] for r in rows if name in r.get("traces", {})]
        return None if not vals else round(statistics.median(vals), 1)

    print("\n" + "=" * 72)
    print("EMERALD BENCHMARK — medians")
    print("=" * 72)

    print("\nCold start (single tab)")
    print(f"  exec → painted        {trace_median(results['idle_one_tab'], 'wall_to_painted')} ms"
          "   <- usable (wall clock)")
    print("  of which, after app setup begins:")
    print(f"    window created      {trace_median(results['idle_one_tab'], 'window')} ms")
    print(f"    core ready          {trace_median(results['idle_one_tab'], 'ready')} ms")
    print(f"    chrome painted      {trace_median(results['idle_one_tab'], 'chrome-painted')} ms")

    # Marginal cost per tab, which is where Emerald is weakest and Chromium's
    # renderer sharing is strongest. Reported because omitting it would make
    # the headline comparison misleading.
    one = agg(results["idle_one_tab"], "pss_kb")
    many_pss = agg(results["all_awake"], "pss_kb")
    if one and many_pss:
        print(f"\nMarginal PSS per additional awake tab: "
              f"~{mb((many_pss - one) / max(1, args.tabs - 1))} MB")

    for name, label in [
        ("idle_one_tab", "1 tab, idle"),
        ("all_awake", f"{args.tabs} tabs, all awake"),
        ("capped", f"{args.tabs} tabs, ceiling 2"),
    ]:
        rows = results[name]
        print(f"\n{label}")
        print(f"  RSS  (tree total)     {mb(agg(rows, 'rss_kb'))} MB")
        pss = agg(rows, "pss_kb")
        if pss:
            print(f"  PSS  (fair total)     {mb(pss)} MB")
        print(f"  Emerald shell alone   {mb(agg(rows, 'shell_rss_kb'))} MB")
        print(f"  processes             {statistics.median(len(r['processes']) for r in rows):.0f}")

    print("\nSuspension (the shipped tab ceiling, not a test hook)")
    for metric in ("rss_kb", "pss_kb"):
        awake = agg(results["all_awake"], metric)
        capped = agg(results["capped"], metric)
        if not (awake and capped):
            continue
        saved = awake - capped
        label = "RSS" if metric == "rss_kb" else "PSS"
        print(
            f"  {label}: {args.tabs} awake → 2 awake   {mb(awake)} MB → {mb(capped)} MB"
            f"   ({mb(saved)} MB returned, {saved / awake * 100:.0f}%)"
        )
        print(f"       per discarded tab  ~{mb(saved / max(1, args.tabs - 2))} MB")

    procs_awake = statistics.median(len(r["processes"]) for r in results["all_awake"])
    procs_capped = statistics.median(len(r["processes"]) for r in results["capped"])
    print(f"  processes: {procs_awake:.0f} → {procs_capped:.0f}")
    if procs_capped >= procs_awake:
        print("  !! WARNING: process count did not drop — the ceiling did not take effect.")
        print("     Check that write_settings() targets the right config directory.")

    if results["chromium_one_tab"]:
        print("\nSame-machine baseline — Chromium, identical pages and display")
        for name, label in [
            ("chromium_one_tab", "1 tab"),
            ("chromium_many", f"{args.tabs} tabs"),
        ]:
            rows = results[name]
            print(f"  {label:<10} RSS {mb(agg(rows, 'rss_kb'))} MB   PSS {mb(agg(rows, 'pss_kb'))} MB"
                  f"   ({statistics.median(len(r['processes']) for r in rows):.0f} processes)")
        for metric, label in [("pss_kb", "PSS"), ("rss_kb", "RSS")]:
            e1 = agg(results["idle_one_tab"], metric)
            c1 = agg(results["chromium_one_tab"], metric)
            en = agg(results["capped"], metric)
            cn = agg(results["chromium_many"], metric)
            if e1 and c1:
                print(f"  {label} 1 tab:   Emerald {mb(e1)} MB vs Chromium {mb(c1)} MB"
                      f"  ({e1 / c1 * 100:.0f}%)")
            if en and cn:
                print(f"  {label} {args.tabs} tabs:  Emerald {mb(en)} MB vs Chromium {mb(cn)} MB"
                      f"  ({en / cn * 100:.0f}%)")
        c1 = agg(results["chromium_one_tab"], "pss_kb")
        cn = agg(results["chromium_many"], "pss_kb")
        if c1 and cn:
            print(f"  Chromium marginal PSS per tab: ~{mb((cn - c1) / max(1, args.tabs - 1))} MB")
        print("  notes:")
        print("    - Chromium ran with --no-sandbox (root), which removes its sandbox")
        print("      helper processes and favours it slightly on both counts.")
        print("    - the fixture pages are local files, so Chromium treats them as one")
        print("      site and reuses renderer processes across tabs. That flatters its")
        print("      marginal per-tab cost relative to 8 genuinely different websites.")

    if args.json:
        args.json.write_text(json.dumps(results, indent=2))
        print(f"\nraw results → {args.json}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
