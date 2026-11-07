//! Noticing what happens to the presences folder from outside the application.

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use rpp_plugins::Host;

use crate::controller::{Controller, lock};

/// Each folder of a place, with when it last changed.
pub(crate) type FolderState = Vec<(OsString, Option<SystemTime>)>;

/// Reads what is in a place. Hidden entries are left out: staging folders come and go.
pub(crate) fn state_of(root: &Path) -> FolderState {
    let Ok(entries) = fs::read_dir(root) else {
        return FolderState::new();
    };
    let mut state: FolderState = entries
        .flatten()
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| {
            let changed = entry.metadata().and_then(|meta| meta.modified()).ok();
            (entry.file_name(), changed)
        })
        .collect();
    state.sort();
    state
}

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// Reads the presences again if their folder changed from outside.
    ///
    /// Returns:
    ///     True when something changed and the window and the tray should read again.
    pub fn notice_folder_changes(&self) -> bool {
        let Some(root) = self.roots.first() else {
            return false;
        };
        if *lock(&self.folders_seen) == state_of(root) {
            return false;
        }
        self.reload_presences();
        true
    }
}
