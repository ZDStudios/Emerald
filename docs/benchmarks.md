# Benchmarks

Every number here was measured on this machine with the shipped release binary. Nothing is estimated. The harness is `bench/bench.py`, the raw output is `bench/results.json`, and the whole thing reruns with:

```bash
CHROMIUM=/path/to/chrome ./bench/run.sh --tabs 8 --repeat 3
```

## Method, and why it is shaped this way

**No test-only code paths.** Tabs are opened by passing URLs on the command line — a real feature — and suspension is triggered by writing a real `settings.json` with a low `max_live_tabs` and letting the ordinary policy run. If these numbers are right, the shipped behaviour is what produced them. There is no benchmark mode.

**The whole process tree is measured.** A browser is not one process: Emerald is the shell plus one `WebKitWebProcess` per live webview plus a shared network process. Measuring only the shell would flatter the result enormously. `bench.py` walks `/proc` from the browser's pid and sums every descendant, using the same arithmetic as `src-tauri/src/metrics.rs`.

**RSS and PSS are both reported, and PSS is the honest one.** RSS counts every physical page a process has mapped, *including pages shared with other processes* — so a tree total counts the engine library once per web process. PSS divides each shared page by the number of sharers. Emerald's entire argument (not shipping an engine) shows up in PSS and is nearly invisible in RSS.

**Samples wait for the tree to settle.** Memory right after launch is still climbing. The harness polls until two consecutive samples are within 2MB of each other rather than sleeping for an arbitrary interval.

**Medians of 3 runs**, one Xvfb server shared across every scenario so the display is identical.

### Environment

| | |
| --- | --- |
| OS | Ubuntu 24.04, Linux 6.18 |
| Engine | WebKitGTK 2.52.3 |
| Display | Xvfb 1280×820×24, **software rendering** |
| Build | `--release`, LTO, `opt-level = "s"`, stripped |
| Pages | 3 local HTML fixtures in `bench/pages/`, cycled |

---

> **Numbers below predate the extensions, bookmarks-bar and Chrome-layout work.**
> The binary grew from 6.0 MB to 7.2 MB and the chrome bundle from 75 KB to 86 KB
> in that change, so cold start in particular is likely to have moved. They are
> left in place rather than quietly adjusted, and re-running `./bench/run.sh`
> regenerates them honestly.

## Results

### Cold start

| Milestone | Median |
| --- | --- |
| **exec → chrome painted** | **1277 ms** |
| ...window created (from setup) | 7.4 ms |
| ...core ready (from setup) | 27.5 ms |
| ...chrome painted (from setup) | 1146 ms |

The headline is wall clock, from just before `Popen` to the moment the chrome reports it has painted. It includes process exec, dynamic linking and GTK init, which Emerald's own instrumentation cannot see from inside `setup()`.

**This is slower than it should be and it is the weakest number here.** The Rust core is ready in 27ms; the remaining ~1.1s is the chrome webview loading and rendering its own UI. That has not been attributed further — it could be bundle parse, Solid's first render, the initial `get_state` round trip, WebKit's first-paint path under software rendering, or some combination. Guessing in a benchmark document would be worse than saying so. It is the obvious next thing to profile.

> An earlier draft of this file reported **261 ms**. That number was wrong: the binary had been built with plain `cargo build --release`, which omits the `custom-protocol` feature, so the chrome tried to load from the dev server at `localhost:1420`, failed, and painted an error page. It started fast because its entire UI had failed to load. The measurement was real; what it measured was not Emerald. `bench/run.sh` now builds correctly, and this is a good argument for screenshotting anything you benchmark.

### Memory

| Scenario | RSS | **PSS** | Processes |
| --- | --- | --- | --- |
| 1 tab, idle | 514 MB | **235 MB** | 4 |
| 8 tabs, all awake | 1579 MB | **405 MB** | 11 |
| 8 tabs, ceiling of 2 | 675 MB | **270 MB** | 5 |

Marginal cost of each additional awake tab: **~24.3 MB PSS**.

### Suspension actually returns memory

Going from 8 awake tabs to a ceiling of 2, using the shipped policy:

| | Before | After | Returned |
| --- | --- | --- | --- |
| RSS | 1579 MB | 675 MB | **904 MB (57%)** |
| PSS | 405 MB | 270 MB | **135 MB (33%)** |
| Processes | 11 | 5 | **6 web processes terminated** |

**The process count is the point.** Six `WebKitWebProcess` instances were terminated, not hidden, throttled, or visually collapsed. The pages are gone from memory and reload on click. ~22.5 MB PSS per discarded tab.

---

## Same-machine baseline: Chromium

Chromium 1194 (Playwright build), same Xvfb display, same local pages, same harness, same sampling code.

| | Emerald PSS | Chromium PSS | Emerald as % |
| --- | --- | --- | --- |
| 1 tab | 235 MB | 370 MB | **64%** |
| 8 tabs (Emerald capped at 2) | 270 MB | 448 MB | **60%** |

| | Emerald RSS | Chromium RSS | Emerald as % |
| --- | --- | --- | --- |
| 1 tab | 514 MB | 875 MB | **59%** |
| 8 tabs (Emerald capped at 2) | 675 MB | 1607 MB | **42%** |

Processes: Emerald 4 vs Chromium 8 at one tab; Emerald 5 (capped) vs Chromium 15 at eight.

### Three caveats that cut against Emerald

These matter more than the headline, so they are not in a footnote.

1. **Chromium's marginal per-tab cost is lower than Emerald's** — ~11.1 MB PSS per tab versus Emerald's ~24.3 MB. Chromium's renderer sharing and its more aggressive process model genuinely beat WebKitGTK's here. **Emerald wins on the floor, not on the slope.** What keeps Emerald's totals down at high tab counts is the discard policy, not per-tab efficiency. Take the policy away and the advantage narrows considerably.

2. **The fixture flatters Chromium's slope further.** All 8 tabs load `file://` URLs, which Chromium treats as one site and consolidates into shared renderers. Against 8 genuinely different websites its process count and marginal cost would both be higher. This makes point 1 a conservative statement of Emerald's disadvantage, not an overstatement — but it also means the marginal numbers are not a clean comparison in either direction.

3. **Chromium ran with `--no-sandbox`** because the harness runs as root. That removes its sandbox helper processes, so its process count and memory are both slightly *understated*.

### What this comparison is not

It is not a comparison against Chrome or Arc as shipped. Those carry extensions, sync, profile services, and a UI layer that this Chromium build does not, and they would measure worse. Quoting a number for "Chrome" that was actually measured on a bare Chromium would be dishonest, so the table above says Chromium and means it.

It is also a single machine, on Linux, under software rendering. **Xvfb software rendering is not representative of a real GPU** and inflates both browsers' numbers in ways that may not be symmetric. macOS and Windows numbers have not been collected.

---

## Other measurements

| | |
| --- | --- |
| Binary | **7.2 MB** — no bundled engine |
| Chrome bundle | 86 KB JS + 60 KB CSS (29 KB + 27 KB gzipped) |
| Bundled fonts | 480 KB woff2, of which OpenDyslexic is 236 KB |
| Rust unit tests | 49, ~0.00s |
| Background timers at rest | **0** |

That last row is checkable rather than rhetorical: with `suspend_idle_minutes`, `auto_archive_idle_minutes` and `memory_budget_mb` all zero, `Settings::needs_sampler()` returns false and the policy thread parks on a condvar. There is no other periodic work in the program.

---

## Honest reading of all this

**What is well supported:** Emerald's idle floor is meaningfully lower than a comparable Chromium's on the same machine — roughly 64% of its PSS with one tab. The suspension mechanism does what it claims: processes die and memory returns.

**What is not supported by these numbers:** any claim that Emerald is more memory-efficient *per tab*. It is not; it is worse, and the discard policy is what compensates. Any claim about Chrome or Arc specifically. Any claim about macOS or Windows. Any claim about behaviour over hours of real browsing — every measurement here is of a browser that has been open for seconds. And nothing here says Emerald starts quickly: at 1.28s it does not.

**The number most likely to be wrong in practice** is the 235 MB idle figure, because real browsing accumulates caches that a ten-second benchmark never allocates. A long-session measurement is the obvious next thing to build.
