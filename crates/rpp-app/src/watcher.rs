//! Watching one presence folder, so a saved edit applies once it settles.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

/// Time between two looks at the folder.
const POLL: Duration = Duration::from_millis(250);
/// Quiet time after the last edit before it is reported.
const SETTLE: Duration = Duration::from_millis(500);
/// Most files looked at; more means the folder is not a presence.
const MAX_FILES: usize = 512;

/// Path, modification time and size of every file that matters.
pub type Fingerprint = Vec<(PathBuf, SystemTime, u64)>;

/// Reads the fingerprint of a presence folder.
///
/// Raises:
///     String: The folder cannot be read or holds more than 512 files.
pub fn fingerprint(folder: &Path) -> Result<Fingerprint, String> {
    let mut found = Fingerprint::new();
    let mut pending = vec![folder.to_owned()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if name.starts_with('.') || name == "node_modules" || kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
            } else if is_watched(&path) {
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                found.push((
                    path,
                    meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    meta.len(),
                ));
                if found.len() > MAX_FILES {
                    return Err(format!("more than {MAX_FILES} files"));
                }
            }
        }
    }
    found.sort();
    Ok(found)
}

fn is_watched(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("js") || extension.eq_ignore_ascii_case("json")
        })
}

/// A thread that watches a folder until it is dropped.
pub struct Watcher {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Watcher {
    /// Starts watching a presence folder.
    ///
    /// Args:
    ///     changed: Called once edits settle; returns false to stop watching,
    ///         for instance when the application is closing.
    pub fn start(folder: PathBuf, changed: impl Fn() -> bool + Send + 'static) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("presence-watcher".into())
            .spawn(move || watch(&folder, &flag, &changed))
            .inspect_err(|error| tracing::warn!("changes will not reload by themselves: {error}"))
            .ok();
        Self { stop, thread }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            // The last owner of the controller can be the watcher thread itself
            // (it holds the controller while it reloads); it cannot wait for itself.
            if thread.thread().id() != thread::current().id() {
                let _ = thread.join();
            }
        }
    }
}

/// Decides when a series of looks at a folder is one finished edit.
#[derive(Debug, Default)]
struct Settling {
    /// What the folder held after the last reported change.
    settled: Option<Fingerprint>,
    /// A different state, and when it was first seen.
    pending: Option<(Fingerprint, Instant)>,
}

impl Settling {
    /// Takes one look at what the folder holds now. True once for each edit
    /// that finished.
    fn look(&mut self, current: Fingerprint, now: Instant) -> bool {
        let Some(before) = &self.settled else {
            self.settled = Some(current);
            return false;
        };
        if *before == current {
            self.pending = None;
            return false;
        }
        match &self.pending {
            Some((seen, since)) if *seen == current => {
                if now.duration_since(*since) < SETTLE {
                    return false;
                }
                self.settled = Some(current);
                self.pending = None;
                true
            }
            _ => {
                self.pending = Some((current, now));
                false
            }
        }
    }
}

fn watch(folder: &Path, stop: &AtomicBool, changed: &dyn Fn() -> bool) {
    let mut settling = Settling {
        settled: fingerprint(folder).ok(),
        pending: None,
    };
    loop {
        thread::park_timeout(POLL);
        if stop.load(Ordering::Relaxed) {
            return;
        }
        // A folder that cannot be read for a moment (an editor replacing a
        // file) is not a change; the next look decides.
        let Ok(current) = fingerprint(folder) else {
            continue;
        };
        if settling.look(current, Instant::now()) && !changed() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    fn write(folder: &Path, name: &str, text: &str) {
        let path = folder.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn only_scripts_and_json_files_count() {
        let folder = tempfile::tempdir().unwrap();
        write(folder.path(), "main.js", "a");
        write(folder.path(), "manifest.json", "{}");
        write(folder.path(), "locales/en.json", "{}");
        write(folder.path(), "notes.txt", "ignored");
        write(folder.path(), ".hidden/secret.js", "ignored");
        write(folder.path(), "node_modules/lib/index.js", "ignored");
        let names: Vec<_> = fingerprint(folder.path())
            .unwrap()
            .iter()
            .map(|(path, ..)| path.strip_prefix(folder.path()).unwrap().to_owned())
            .collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&PathBuf::from("locales").join("en.json")));
    }

    #[test]
    fn an_edit_changes_the_fingerprint_and_a_missing_folder_is_an_error() {
        let folder = tempfile::tempdir().unwrap();
        write(folder.path(), "main.js", "one");
        let before = fingerprint(folder.path()).unwrap();
        write(folder.path(), "main.js", "longer text");
        assert_ne!(fingerprint(folder.path()).unwrap(), before);
        assert!(fingerprint(&folder.path().join("missing")).is_err());
    }

    #[test]
    fn a_folder_with_too_many_files_is_refused() {
        let folder = tempfile::tempdir().unwrap();
        for number in 0..=MAX_FILES {
            write(folder.path(), &format!("f{number}.js"), "");
        }
        assert!(fingerprint(folder.path()).is_err());
    }

    fn counting(folder: &Path) -> (Watcher, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        let watcher = Watcher::start(folder.to_owned(), move || {
            seen.fetch_add(1, Ordering::SeqCst);
            true
        });
        // Let the first look record the starting state.
        thread::sleep(Duration::from_millis(400));
        (watcher, count)
    }

    fn state(size: u64) -> Fingerprint {
        vec![(PathBuf::from("main.js"), SystemTime::UNIX_EPOCH, size)]
    }

    /// With the clock passed in, a slow machine cannot change the outcome.
    #[test]
    fn a_burst_of_saves_is_reported_once_after_it_settles() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut settling = Settling::default();
        assert!(
            !settling.look(state(1), at(0)),
            "the first look only records"
        );

        // Each save restarts the wait.
        assert!(!settling.look(state(2), at(100)));
        assert!(!settling.look(state(3), at(200)));
        assert!(!settling.look(state(4), at(300)));
        assert!(!settling.look(state(4), at(799)));
        assert!(settling.look(state(4), at(800)));

        // Nothing changed since: it stays quiet.
        assert!(!settling.look(state(4), at(2000)));
    }

    #[test]
    fn going_back_to_the_saved_state_reports_nothing() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut settling = Settling::default();
        settling.look(state(1), at(0));
        assert!(!settling.look(state(2), at(100)));
        assert!(!settling.look(state(1), at(200)));
        assert!(!settling.look(state(2), at(900)), "the wait starts over");
    }

    #[test]
    fn a_saved_edit_reaches_the_owner_of_the_watch() {
        let folder = tempfile::tempdir().unwrap();
        write(folder.path(), "main.js", "start");
        let (_watcher, count) = counting(folder.path());
        write(folder.path(), "main.js", "saved");
        for _ in 0..100 {
            if count.load(Ordering::SeqCst) > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn dropping_the_watcher_stops_it() {
        let folder = tempfile::tempdir().unwrap();
        write(folder.path(), "main.js", "start");
        let (watcher, count) = counting(folder.path());
        drop(watcher);
        write(folder.path(), "main.js", "changed after the drop");
        thread::sleep(Duration::from_millis(1200));
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn answering_false_ends_the_watch() {
        let folder = tempfile::tempdir().unwrap();
        write(folder.path(), "main.js", "start");
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        let _watcher = Watcher::start(folder.path().to_owned(), move || {
            seen.fetch_add(1, Ordering::SeqCst);
            false
        });
        thread::sleep(Duration::from_millis(400));
        write(folder.path(), "main.js", "first edit");
        thread::sleep(Duration::from_millis(1300));
        write(folder.path(), "main.js", "second edit, longer");
        thread::sleep(Duration::from_millis(1300));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
