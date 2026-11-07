//! The developer tools that keep files: versions, test cases and packages.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rpp_plugins::Host;
use rpp_service::Presence;

use crate::cases::{self, Case};
use crate::contribution;
use crate::controller::{Controller, lock};
use crate::library::effective_language;
use crate::versions;
use crate::views::{CaseResult, FileChange, PreparedView, ReviewCheck, VersionView};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// The folder of a presence and where its tools keep their files.
    ///
    /// Raises:
    ///     String: The presence does not exist or the user did not write it.
    fn bench(&self, name: &str) -> Result<(PathBuf, PathBuf), String> {
        let folder = self.local_folder(name)?;
        let key = folder.file_name().ok_or("The presence has no folder.")?;
        let dir = self.data_dir.join("workbench").join(key);
        Ok((folder, dir))
    }

    fn prepared_presence(&self, name: &str) -> Result<Presence, String> {
        let settings = lock(&self.settings);
        let library = lock(&self.library);
        let language = effective_language(&settings, &self.system_language);
        let presence = library
            .find(name)
            .ok_or_else(|| format!("Unknown presence: {name}"))?;
        Ok(library.prepared(presence, &settings, &language))
    }

    fn declared_version(&self, name: &str) -> Result<String, String> {
        lock(&self.library)
            .find(name)
            .map(|presence| presence.manifest.version.clone())
            .ok_or_else(|| format!("Unknown presence: {name}"))
    }

    /// The saved versions of a presence, newest first.
    pub fn presence_versions(&self, name: &str) -> Result<Vec<VersionView>, String> {
        let (_, dir) = self.bench(name)?;
        Ok(versions::list(&dir))
    }

    /// Saves a presence as a new version, with `notes` on what changed.
    ///
    /// Raises:
    ///     String: Nothing changed since the last version, or a file cannot be
    ///         read or written.
    pub fn save_version(&self, name: &str, notes: &str) -> Result<VersionView, String> {
        let (folder, dir) = self.bench(name)?;
        let version = self.declared_version(name)?;
        let _busy = lock(&self.workbench);
        versions::save(&dir, &folder, &version, notes, now())
    }

    /// What changed in a presence since a saved version.
    pub fn compare_version(&self, name: &str, id: u32) -> Result<Vec<FileChange>, String> {
        let (folder, dir) = self.bench(name)?;
        versions::compare(&dir, id, &folder)
    }

    /// Puts a saved version back and reloads the presence.
    ///
    /// Raises:
    ///     String: The version is not saved or is damaged, a file cannot be
    ///         written, or what came back does not load.
    pub fn restore_version(&self, name: &str, id: u32) -> Result<(), String> {
        let (folder, dir) = self.bench(name)?;
        let version = self.declared_version(name)?;
        {
            let _busy = lock(&self.workbench);
            versions::restore(&dir, id, &folder, &version, now())?;
        }
        self.read_changed(&folder)
    }

    /// The test cases of a presence.
    pub fn test_cases(&self, name: &str) -> Result<Vec<Case>, String> {
        let (_, dir) = self.bench(name)?;
        Ok(cases::load(&dir))
    }

    /// Keeps a test case, replacing the one with the same name.
    pub fn save_test_case(&self, name: &str, case: Case) -> Result<(), String> {
        let (_, dir) = self.bench(name)?;
        let _busy = lock(&self.workbench);
        cases::put(&dir, case)
    }

    /// Removes a test case.
    pub fn delete_test_case(&self, name: &str, case: &str) -> Result<(), String> {
        let (_, dir) = self.bench(name)?;
        let _busy = lock(&self.workbench);
        cases::remove(&dir, case)
    }

    /// Replays the test cases of a presence as it is now.
    pub fn run_test_cases(&self, name: &str) -> Result<Vec<CaseResult>, String> {
        let (_, dir) = self.bench(name)?;
        let saved = cases::load(&dir);
        let presence = self.prepared_presence(name)?;
        Ok(cases::run(&presence, &saved))
    }

    /// Every rule of the catalog, with what to fix in the ones the presence
    /// does not follow.
    pub fn review_presence(&self, name: &str) -> Result<Vec<ReviewCheck>, String> {
        let (folder, _) = self.bench(name)?;
        contribution::review(&folder)
    }

    /// Writes a clean copy of a presence to the exports folder, ready to upload.
    ///
    /// Raises:
    ///     String: It breaks a rule of the catalog, or the copy cannot be written.
    pub fn prepare_presence(&self, name: &str) -> Result<PreparedView, String> {
        let (folder, _) = self.bench(name)?;
        contribution::prepare(&folder, &self.data_dir.join("exports"))
    }
}
