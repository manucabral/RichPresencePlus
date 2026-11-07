//! The log file of the application.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Registry, fmt, reload};

/// Days of logs kept.
const KEPT_FILES: usize = 7;
/// Most lines returned to the window.
const MAX_LINES: usize = 500;
/// Modules whose messages follow the level the user chose.
const OURS: [&str; 7] = [
    "rpp_app",
    "rpp_bridge",
    "rpp_catalog",
    "rpp_core",
    "rpp_desktop",
    "rpp_plugins",
    "rpp_service",
];

/// The running logger.
pub struct Logging {
    filter: reload::Handle<Targets, Registry>,
    folder: PathBuf,
    // Flushes the background writer when the application ends.
    _guard: WorkerGuard,
}

/// The last lines of the log, for the settings page.
#[derive(Debug, Serialize)]
pub struct LogView {
    /// Folder that holds the files.
    pub folder: String,
    /// The last lines of the newest file, oldest first.
    pub lines: Vec<String>,
}

fn level(name: &str) -> LevelFilter {
    match name {
        "error" => LevelFilter::ERROR,
        "warn" => LevelFilter::WARN,
        "debug" => LevelFilter::DEBUG,
        _ => LevelFilter::INFO,
    }
}

fn targets(name: &str) -> Targets {
    let ours = level(name);
    OURS.iter().fold(
        Targets::new().with_default(LevelFilter::WARN),
        |targets, crate_name| targets.with_target(*crate_name, ours),
    )
}

impl Logging {
    /// Starts writing the log in the `logs` folder of `data_dir`.
    ///
    /// Args:
    ///     level_name: `error`, `warn`, `info` or `debug`.
    ///
    /// Returns:
    ///     `None` if the folder cannot be used or another logger is already
    ///     installed. The application runs either way.
    pub fn start(data_dir: &Path, level_name: &str) -> Option<Self> {
        let folder = data_dir.join("logs");
        let appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("rpp")
            .filename_suffix("log")
            .max_log_files(KEPT_FILES)
            .build(&folder)
            .ok()?;
        let (writer, guard) = tracing_appender::non_blocking(appender);
        let (filter, handle) = reload::Layer::new(targets(level_name));
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(writer).with_ansi(false))
            .try_init()
            .ok()?;
        Some(Self {
            filter: handle,
            folder,
            _guard: guard,
        })
    }

    /// Changes the detail without restarting: `error`, `warn`, `info` or `debug`.
    pub fn set_level(&self, level_name: &str) {
        let _ = self.filter.reload(targets(level_name));
    }

    /// The folder and up to the last 500 lines of the newest log file.
    pub fn view(&self) -> LogView {
        let folder = self.folder.to_string_lossy().into_owned();
        let newest = rpp_host::files::latest(&folder, ".log")
            .ok()
            .and_then(|path| path.as_str().map(str::to_owned));
        let lines = newest
            .and_then(|path| rpp_host::files::tail(&path, MAX_LINES).ok())
            .and_then(|value| match value {
                Value::Array(lines) => Some(
                    lines
                        .into_iter()
                        .filter_map(|line| line.as_str().map(str::to_owned))
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();
        LogView { folder, lines }
    }
}

/// What the log says about a panic: the thread and the place in the code,
/// never the message, which can hold a page title.
fn panic_note(thread: Option<&str>, place: Option<&std::panic::Location<'_>>) -> String {
    let thread = thread.unwrap_or("unnamed");
    match place {
        Some(place) => format!(
            "thread {thread} panicked at {}:{}",
            place.file(),
            place.line()
        ),
        None => format!("thread {thread} panicked"),
    }
}

/// The note for a panic happening on the current thread.
fn describe(info: &std::panic::PanicHookInfo<'_>) -> String {
    panic_note(std::thread::current().name(), info.location())
}

/// Writes every panic to the log, then runs the hook that was there before.
pub fn log_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("{}", describe(info));
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_note_names_the_thread_and_the_place_but_not_the_message() {
        let place = std::panic::Location::caller();
        let note = panic_note(Some("presence-YouTube"), Some(place));
        assert!(
            note.starts_with("thread presence-YouTube panicked at "),
            "{note}"
        );
        assert!(note.contains("logging.rs:"), "{note}");
        assert_eq!(panic_note(None, None), "thread unnamed panicked");
    }

    #[test]
    fn the_hook_keeps_the_panic_message_out_of_what_it_writes() {
        use std::sync::{Arc, Mutex};
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let writer = Arc::clone(&seen);
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            writer.lock().unwrap().push(describe(info));
        }));
        let result = std::thread::Builder::new()
            .name("presence-Test".into())
            .spawn(|| panic!("secret page title"))
            .unwrap()
            .join();
        std::panic::set_hook(previous);
        assert!(result.is_err());
        let notes = seen.lock().unwrap().clone();
        assert_eq!(notes.len(), 1);
        assert!(notes[0].starts_with("thread presence-Test panicked at"));
        assert!(!notes[0].contains("secret"));
    }
}
