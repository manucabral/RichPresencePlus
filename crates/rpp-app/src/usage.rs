//! Counting the time each presence stays on display, for the "Your week" page.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rpp_core::usage::Usage;
use rpp_plugins::Host;

use crate::controller::{Controller, lock};
use crate::views::UsageView;

/// Name of the file, in the data folder.
const FILE: &str = "usage.json";
/// Largest file read: far above 56 days of real use.
const MAX_FILE: u64 = 2 * 1024 * 1024;
/// How often a changed count is written.
const SAVE_EVERY: Duration = Duration::from_secs(60);
/// A gap longer than this is a computer that slept, not time on display.
const LONGEST_STEP: Duration = Duration::from_secs(30);

/// The count, and what is needed to keep it up to date.
#[derive(Debug, Default)]
pub(crate) struct Tracker {
    /// Read from the disk the first time it is needed; off, it takes no memory.
    usage: Option<Usage>,
    /// The last look, to measure the time since.
    last_look: Option<Instant>,
    /// The last write.
    last_save: Option<Instant>,
    /// There is time the disk does not have yet.
    dirty: bool,
}

fn path_in(folder: &Path) -> PathBuf {
    folder.join(FILE)
}

/// Reads the saved count. A file that is too large or not readable counts as empty.
fn read(folder: &Path) -> Usage {
    let path = path_in(folder);
    if fs::metadata(&path).map_or(true, |meta| meta.len() > MAX_FILE) {
        return Usage::default();
    }
    fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Writes the count, replacing the file in one step.
fn write(folder: &Path, usage: &Usage) -> std::io::Result<()> {
    fs::create_dir_all(folder)?;
    let path = path_in(folder);
    let temporary = path.with_file_name("usage.json.tmp");
    fs::write(&temporary, serde_json::to_vec(usage)?)?;
    fs::rename(temporary, path)
}

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// Adds the time since the last look to the presence on display.
    ///
    /// Args:
    ///     day: Today as `YYYY-MM-DD`; `None` when the clock cannot be read.
    pub fn track_time(&self, day: Option<&str>, now: Instant) {
        if !lock(&self.settings).track_time {
            return;
        }
        let status = self.service.status();
        let delivered = status.activity.is_some() && status.delivery_error.is_none();
        let shown = status
            .rows
            .iter()
            .find(|row| row.selected)
            .filter(|_| delivered)
            .map(|row| row.label.as_str());

        let mut tracker = lock(&self.tracker);
        let step = tracker
            .last_look
            .map(|last| now.saturating_duration_since(last))
            .filter(|step| *step <= LONGEST_STEP);
        tracker.last_look = Some(now);
        let (Some(step), Some(day), Some(name)) = (step, day, shown) else {
            return;
        };
        let usage = tracker.usage.get_or_insert_with(|| read(&self.data_dir));
        usage.add(day, name, step.as_secs());
        tracker.dirty = true;
        let due = tracker
            .last_save
            .is_none_or(|last| now.saturating_duration_since(last) >= SAVE_EVERY);
        if due {
            self.save_usage(&mut tracker, now);
        }
    }

    /// Writes what the disk does not have yet.
    fn save_usage(&self, tracker: &mut Tracker, now: Instant) {
        tracker.last_save = Some(now);
        let Some(usage) = tracker.usage.as_ref().filter(|_| tracker.dirty) else {
            return;
        };
        match write(&self.data_dir, usage) {
            Ok(()) => tracker.dirty = false,
            Err(error) => tracing::warn!("cannot save the time on display: {error}"),
        }
    }

    /// Saves the count before the application closes.
    pub(crate) fn flush_usage(&self) {
        self.save_usage(&mut lock(&self.tracker), Instant::now());
    }

    /// Returns what the "Your week" page shows.
    pub fn usage(&self) -> UsageView {
        let on = lock(&self.settings).track_time;
        let mut tracker = lock(&self.tracker);
        let days = tracker
            .usage
            .get_or_insert_with(|| read(&self.data_dir))
            .days()
            .clone();
        UsageView { on, days }
    }

    /// Turns the count on or off. What was counted stays until it is deleted.
    pub fn set_track_time(&self, on: bool) -> Result<(), String> {
        {
            let mut settings = lock(&self.settings);
            let mut changed = settings.clone();
            changed.track_time = on;
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
        }
        if !on {
            self.flush_usage();
        }
        // The first look after a change only sets the start.
        lock(&self.tracker).last_look = None;
        Ok(())
    }

    /// The count as comma-separated text.
    pub fn export_usage(&self) -> String {
        lock(&self.tracker)
            .usage
            .get_or_insert_with(|| read(&self.data_dir))
            .csv()
    }

    /// Deletes everything counted so far.
    pub fn clear_usage(&self) -> Result<(), String> {
        let mut tracker = lock(&self.tracker);
        tracker.usage = Some(Usage::default());
        tracker.dirty = false;
        match fs::remove_file(path_in(&self.data_dir)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.to_string()),
            _ => Ok(()),
        }
    }
}
