//! Memory and processor time of the application and of its WebView.

use std::collections::{BTreeSet, HashMap, VecDeque};

/// Executable of the WebView2 processes.
pub const WEBVIEW_EXE: &str = "msedgewebview2.exe";

/// One process as the system lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEntry {
    /// Process identifier.
    pub pid: u32,
    /// Identifier of the process that started it.
    pub parent: u32,
    /// Executable file name.
    pub name: String,
}

/// What a group of processes uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
pub struct Usage {
    /// Working set, in bytes.
    pub memory_bytes: u64,
    /// Processor time since each process started, in seconds.
    pub cpu_seconds: f64,
    /// How many processes were measured.
    pub processes: usize,
}

impl Usage {
    fn add(&mut self, memory_bytes: u64, cpu_seconds: f64) {
        self.memory_bytes += memory_bytes;
        self.cpu_seconds += cpu_seconds;
        self.processes += 1;
    }
}

/// The application and its WebView, measured apart.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
pub struct AppUsage {
    /// This process.
    pub app: Usage,
    /// The WebView2 processes under it.
    pub webview: Usage,
}

/// Processes below `root` whose executable is `name`, at any depth.
pub fn descendants(entries: &[ProcessEntry], root: u32, name: &str) -> Vec<u32> {
    let mut children: HashMap<u32, Vec<&ProcessEntry>> = HashMap::new();
    for entry in entries {
        // A process listed as its own parent (pid 0 does it) would loop.
        if entry.pid != entry.parent {
            children.entry(entry.parent).or_default().push(entry);
        }
    }
    let mut seen = BTreeSet::from([root]);
    let mut found = BTreeSet::new();
    let mut queue = VecDeque::from([root]);
    while let Some(pid) = queue.pop_front() {
        for child in children.get(&pid).into_iter().flatten() {
            if seen.insert(child.pid) {
                if child.name.eq_ignore_ascii_case(name) {
                    found.insert(child.pid);
                }
                queue.push_back(child.pid);
            }
        }
    }
    found.into_iter().collect()
}

/// Measures this process and its WebView; `None` off Windows or when the
/// process list cannot be read.
pub fn app_usage() -> Option<AppUsage> {
    #[cfg(windows)]
    {
        let me = std::process::id();
        let entries = crate::win32::process_entries().ok()?;
        let mut usage = AppUsage::default();
        if let Some((memory, cpu)) = crate::win32::process_usage(me) {
            usage.app.add(memory, cpu);
        }
        for pid in descendants(&entries, me, WEBVIEW_EXE) {
            // A process that ended since the list was taken is skipped.
            if let Some((memory, cpu)) = crate::win32::process_usage(pid) {
                usage.webview.add(memory, cpu);
            }
        }
        Some(usage)
    }
    #[cfg(not(windows))]
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(pid: u32, parent: u32, name: &str) -> ProcessEntry {
        ProcessEntry {
            pid,
            parent,
            name: name.into(),
        }
    }

    #[test]
    fn webview_processes_are_found_at_any_depth() {
        let entries = [
            entry(10, 1, "rpp-desktop.exe"),
            entry(20, 10, "msedgewebview2.exe"),
            entry(21, 20, "msedgewebview2.exe"),
            entry(22, 21, "MSEDGEWEBVIEW2.EXE"),
            entry(23, 20, "crashpad.exe"),
            // Another app's WebView is not ours.
            entry(30, 1, "other.exe"),
            entry(31, 30, "msedgewebview2.exe"),
        ];
        assert_eq!(descendants(&entries, 10, WEBVIEW_EXE), [20, 21, 22]);
        assert!(descendants(&entries, 99, WEBVIEW_EXE).is_empty());
    }

    #[test]
    fn reused_identifiers_cannot_loop() {
        let entries = [
            entry(0, 0, "System Idle Process"),
            entry(10, 12, "rpp-desktop.exe"),
            entry(11, 10, "msedgewebview2.exe"),
            // A newer process got the identifier of 10's dead parent.
            entry(12, 11, "msedgewebview2.exe"),
        ];
        assert_eq!(descendants(&entries, 10, WEBVIEW_EXE), [11, 12]);
        assert!(descendants(&entries, 0, WEBVIEW_EXE).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn this_process_can_be_measured() {
        let usage = app_usage().expect("the process list can be read");
        assert_eq!(usage.app.processes, 1);
        assert!(usage.app.memory_bytes > 0);
        assert!(usage.app.cpu_seconds >= 0.0);
    }
}
