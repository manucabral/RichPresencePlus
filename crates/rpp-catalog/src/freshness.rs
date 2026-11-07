//! Refusing an index older than one already accepted.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::Error;
use crate::index::Channel;

/// Largest memory file read.
const MAX_BYTES: u64 = 4096;

/// The newest generation time accepted per channel, kept in a small file.
#[derive(Debug, Clone)]
pub struct Seen {
    path: PathBuf,
    /// Kept as well as the file, so a file that cannot be written does not
    /// reopen the door for the rest of the run.
    accepted: Arc<Mutex<BTreeMap<Channel, u64>>>,
}

impl Seen {
    /// Uses `catalog-seen.json` inside the data folder `directory`.
    pub fn new(directory: &Path) -> Self {
        Self {
            path: directory.join("catalog-seen.json"),
            accepted: Arc::default(),
        }
    }

    fn load(&self) -> BTreeMap<Channel, u64> {
        let usable = fs::metadata(&self.path).is_ok_and(|meta| meta.len() <= MAX_BYTES);
        usable
            .then(|| fs::read_to_string(&self.path).ok())
            .flatten()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Accepts an index generation time, or refuses it when it is older than the last one.
    ///
    /// Args:
    ///     generated_at: Unix seconds.
    ///
    /// Raises:
    ///     Error: `Stale` if an index with a later time was accepted before.
    pub fn accept(&self, channel: Channel, generated_at: u64) -> Result<(), Error> {
        let mut seen = self.load();
        if let Ok(accepted) = self.accepted.lock() {
            for (channel, time) in accepted.iter() {
                let mark = seen.entry(*channel).or_insert(*time);
                *mark = (*mark).max(*time);
            }
        }
        match seen.get(&channel) {
            Some(&latest) if generated_at < latest => return Err(Error::Stale),
            Some(&latest) if generated_at == latest => return Ok(()),
            _ => {}
        }
        seen.insert(channel, generated_at);
        if let Ok(mut accepted) = self.accepted.lock() {
            accepted.insert(channel, generated_at);
        }
        if let Ok(text) = serde_json::to_string(&seen) {
            let temporary = self.path.with_extension("json.tmp");
            if fs::write(&temporary, text).is_ok() && fs::rename(&temporary, &self.path).is_err() {
                let _ = fs::remove_file(&temporary);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_holds_for_the_run_when_the_file_cannot_be_written() {
        let folder = tempfile::tempdir().unwrap();
        // A folder with the name of the temporary file makes every write fail.
        fs::create_dir(folder.path().join("catalog-seen.json.tmp")).unwrap();
        let seen = Seen::new(folder.path());
        assert!(seen.accept(Channel::Stable, 200).is_ok());
        assert!(!folder.path().join("catalog-seen.json").exists());
        assert!(matches!(
            seen.accept(Channel::Stable, 100),
            Err(Error::Stale)
        ));
    }

    #[test]
    fn an_older_index_is_refused_and_a_newer_one_moves_the_mark() {
        let folder = tempfile::tempdir().unwrap();
        let seen = Seen::new(folder.path());
        assert!(seen.accept(Channel::Stable, 100).is_ok());
        assert!(seen.accept(Channel::Stable, 100).is_ok());
        assert!(matches!(
            seen.accept(Channel::Stable, 99),
            Err(Error::Stale)
        ));
        assert!(seen.accept(Channel::Stable, 200).is_ok());
        // The mark survives a restart and each channel has its own.
        let again = Seen::new(folder.path());
        assert!(matches!(
            again.accept(Channel::Stable, 150),
            Err(Error::Stale)
        ));
        assert!(again.accept(Channel::Experimental, 1).is_ok());
    }

    #[test]
    fn a_damaged_memory_file_starts_over() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("catalog-seen.json"), "{not json").unwrap();
        let seen = Seen::new(folder.path());
        assert!(seen.accept(Channel::Stable, 5).is_ok());
        assert!(matches!(seen.accept(Channel::Stable, 4), Err(Error::Stale)));
    }

    #[test]
    fn an_unwritable_folder_does_not_block_updates() {
        let seen = Seen::new(Path::new("Z:/this/folder/does/not/exist"));
        assert!(seen.accept(Channel::Stable, 5).is_ok());
    }
}
