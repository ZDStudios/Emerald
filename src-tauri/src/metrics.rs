//! Resident memory accounting for Emerald and its web processes.
//!
//! Two consumers:
//!
//!   1. the memory-budget policy in `runtime.rs`, which discards tabs when the
//!      whole process tree exceeds `performance.memory_budget_mb`;
//!   2. `bench/`, which reports the numbers quoted in `docs/benchmarks.md`.
//!
//! Emerald is a *tree* of processes, not one process: the shell, plus one
//! WebKitWebProcess per live webview, plus a shared network process. Measuring
//! only the shell would flatter the numbers enormously, so everything here
//! walks the tree and reports the total.
//!
//! ## What is being measured
//!
//! Resident Set Size — physical pages currently mapped. RSS double-counts
//! shared pages across processes (every WebKit process maps the same ~40MB of
//! libwebkit2gtk), so a tree total is an *upper bound* on what Emerald actually
//! costs the machine. Where the kernel offers PSS (Linux, `/proc/pid/smaps_rollup`)
//! we report that too, because PSS divides shared pages by the number of
//! sharers and is the honest number to compare against another browser.
//!
//! No dependency: this is /proc parsing on Linux and `ps` on macOS.

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProcessMemory {
    pub pid: u32,
    /// Best-effort process name, e.g. `emerald` or `WebKitWebProcess`.
    pub name: String,
    pub rss_kb: u64,
    /// Proportional set size — shared pages divided among sharers. `None`
    /// where the platform does not expose it.
    pub pss_kb: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MemorySample {
    /// Sum of RSS over the whole tree. Upper bound; double-counts shared libs.
    pub rss_kb: u64,
    /// Sum of PSS over the whole tree, where available. The fair number.
    pub pss_kb: Option<u64>,
    /// The Emerald shell process alone.
    pub shell_rss_kb: u64,
    /// Every process in the tree, shell first.
    pub processes: Vec<ProcessMemory>,
    /// True when the platform has no implementation here and the numbers are
    /// zeroes rather than measurements. Callers must not silently treat an
    /// unsupported sample as "0 MB used".
    pub unsupported: bool,
}

impl MemorySample {
    pub fn rss_mb(&self) -> f64 {
        self.rss_kb as f64 / 1024.0
    }
    pub fn pss_mb(&self) -> Option<f64> {
        self.pss_kb.map(|kb| kb as f64 / 1024.0)
    }
    pub fn web_process_count(&self) -> usize {
        self.processes.len().saturating_sub(1)
    }
}

/// Sample this process and all of its descendants.
pub fn sample() -> MemorySample {
    let me = std::process::id();
    #[cfg(target_os = "linux")]
    {
        linux::sample_tree(me)
    }
    #[cfg(target_os = "macos")]
    {
        macos::sample_tree(me)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = me;
        MemorySample { unsupported: true, ..Default::default() }
    }
}

/// Descendants of `root`, breadth-first, given a `(pid, ppid)` edge list.
fn descendants(root: u32, edges: &[(u32, u32)]) -> Vec<u32> {
    let mut out = vec![root];
    let mut frontier = vec![root];
    // Bounded so a pathological/looping process table cannot hang the sampler.
    for _ in 0..8 {
        let mut next = Vec::new();
        for (pid, ppid) in edges {
            if frontier.contains(ppid) && !out.contains(pid) {
                out.push(*pid);
                next.push(*pid);
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    out
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    pub fn sample_tree(root: u32) -> MemorySample {
        let edges = read_edges();
        let pids = descendants(root, &edges);
        let page_kb = page_size_kb();

        let mut sample = MemorySample::default();
        for pid in pids {
            let Some(rss_kb) = read_rss_kb(pid, page_kb) else {
                continue; // process exited between listing and reading
            };
            let pss_kb = read_pss_kb(pid);
            if pid == root {
                sample.shell_rss_kb = rss_kb;
            }
            sample.rss_kb += rss_kb;
            if let Some(pss) = pss_kb {
                sample.pss_kb = Some(sample.pss_kb.unwrap_or(0) + pss);
            }
            sample.processes.push(ProcessMemory {
                pid,
                name: read_name(pid),
                rss_kb,
                pss_kb,
            });
        }
        sample
    }

    fn page_size_kb() -> u64 {
        // SAFETY: sysconf with a constant name has no preconditions and cannot
        // fail in a way that matters; a non-positive result falls back to 4KiB.
        let bytes = unsafe { libc_sysconf() };
        if bytes > 0 {
            bytes as u64 / 1024
        } else {
            4
        }
    }

    // Avoids a `libc` dependency for one call.
    unsafe fn libc_sysconf() -> i64 {
        extern "C" {
            fn sysconf(name: i32) -> i64;
        }
        const SC_PAGESIZE: i32 = 30; // Linux value
        sysconf(SC_PAGESIZE)
    }

    fn read_edges() -> Vec<(u32, u32)> {
        let Ok(entries) = std::fs::read_dir("/proc") else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter_map(|e| {
                let pid: u32 = e.file_name().to_str()?.parse().ok()?;
                let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
                // comm can contain spaces and parentheses, so split after the
                // final ')': fields then are state, ppid, ...
                let rest = stat.rsplit_once(')')?.1;
                let ppid: u32 = rest.split_whitespace().nth(1)?.parse().ok()?;
                Some((pid, ppid))
            })
            .collect()
    }

    fn read_rss_kb(pid: u32, page_kb: u64) -> Option<u64> {
        let statm = std::fs::read_to_string(format!("/proc/{pid}/statm")).ok()?;
        let resident_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
        Some(resident_pages * page_kb)
    }

    fn read_pss_kb(pid: u32) -> Option<u64> {
        // smaps_rollup is cheap; smaps is not. Only the rollup is used.
        let rollup = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).ok()?;
        rollup.lines().find_map(|line| {
            let rest = line.strip_prefix("Pss:")?;
            rest.split_whitespace().next()?.parse().ok()
        })
    }

    fn read_name(pid: u32) -> String {
        std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| format!("pid {pid}"))
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    pub fn sample_tree(root: u32) -> MemorySample {
        // No /proc on macOS. `ps` is the portable route that does not require
        // linking libproc or holding task ports for child processes.
        let Ok(out) = std::process::Command::new("ps")
            .args(["-axo", "pid=,ppid=,rss=,comm="])
            .output()
        else {
            return MemorySample { unsupported: true, ..Default::default() };
        };
        let text = String::from_utf8_lossy(&out.stdout);

        let rows: Vec<(u32, u32, u64, String)> = text
            .lines()
            .filter_map(|line| {
                let mut it = line.split_whitespace();
                let pid = it.next()?.parse().ok()?;
                let ppid = it.next()?.parse().ok()?;
                let rss_kb = it.next()?.parse().ok()?;
                let comm = it.collect::<Vec<_>>().join(" ");
                let name = comm.rsplit('/').next().unwrap_or(&comm).to_string();
                Some((pid, ppid, rss_kb, name))
            })
            .collect();

        let edges: Vec<(u32, u32)> = rows.iter().map(|(p, pp, _, _)| (*p, *pp)).collect();
        let pids = descendants(root, &edges);

        let mut sample = MemorySample::default();
        for pid in pids {
            let Some((_, _, rss_kb, name)) = rows.iter().find(|(p, _, _, _)| *p == pid) else {
                continue;
            };
            if pid == root {
                sample.shell_rss_kb = *rss_kb;
            }
            sample.rss_kb += rss_kb;
            sample.processes.push(ProcessMemory {
                pid,
                name: name.clone(),
                rss_kb: *rss_kb,
                pss_kb: None, // macOS exposes no PSS equivalent through ps
            });
        }
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descendants_walks_the_tree_and_ignores_unrelated_processes() {
        // 1 → 2 → 4, 1 → 3; 99 is a stranger.
        let edges = [(2, 1), (3, 1), (4, 2), (99, 100)];
        let mut got = descendants(1, &edges);
        got.sort_unstable();
        assert_eq!(got, vec![1, 2, 3, 4]);
    }

    #[test]
    fn descendants_terminates_on_a_cyclic_process_table() {
        // A malformed/racing table where 2 and 3 claim each other.
        let edges = [(2, 3), (3, 2), (2, 1)];
        let got = descendants(1, &edges);
        assert!(got.contains(&1) && got.contains(&2));
        assert!(got.len() <= 3, "must not loop forever: {got:?}");
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn sampling_this_process_returns_a_plausible_number() {
        let s = sample();
        assert!(!s.unsupported);
        assert!(s.shell_rss_kb > 0, "the test binary occupies memory");
        assert!(s.rss_kb >= s.shell_rss_kb);
        assert!(s.rss_mb() < 100_000.0, "sanity: not terabytes");
    }
}
