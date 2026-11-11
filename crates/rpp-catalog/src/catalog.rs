//! The catalog as the application uses it: list, install, update, remove.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rpp_plugins::Grant;
use semver::Version;
use serde::Serialize;

use crate::error::Error;
use crate::freshness::Seen;
use crate::history;
use crate::index::{Channel, Entry, Index, Keys};
use crate::install::{self, Installed, Standing};
use crate::installs;
use crate::seal::Key;
use crate::source::Fetched;
use crate::source::Source;

/// How long a verified index is reused before it is downloaded again.
const CACHE: Duration = Duration::from_secs(300);

/// How long install counts are reused; the catalog publishes them once a day.
const COUNTS_CACHE: Duration = Duration::from_secs(3600);

/// Wait before asking for the counts again after a download that failed.
const COUNTS_RETRY: Duration = Duration::from_secs(300);

/// Where a catalog presence stands on this computer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum State {
    /// Not installed.
    Available,
    /// Installed and current.
    Installed {
        /// Installed version.
        version: Version,
    },
    /// Installed, but a file was changed afterwards; it is not loaded until reinstalled.
    Modified {
        /// Installed version.
        installed: Version,
    },
    /// Installed, with a newer version in the catalog.
    Update {
        /// Installed version.
        installed: Version,
    },
}

/// A presence of the catalog and its state here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listing {
    /// What the catalog says about it.
    #[serde(flatten)]
    pub entry: Entry,
    /// Whether it is installed, and whether it can be updated.
    #[serde(flatten)]
    pub state: State,
    /// Versions kept from before updates, newest first; only for installed presences.
    pub history: Vec<Version>,
}

struct Cached {
    at: Instant,
    index: Index,
    /// What the server called this copy; asking with it skips an unchanged download.
    tag: Option<String>,
}

/// The install counts last asked for.
struct Counts {
    /// When to ask again.
    until: Instant,
    by_folder: BTreeMap<String, u64>,
    tag: Option<String>,
}

/// The signed catalog of one channel, bound to a presences folder.
pub struct Catalog {
    source: Box<dyn Source>,
    keys: Keys,
    channel: Channel,
    root: PathBuf,
    history: PathBuf,
    key: Key,
    seen: Seen,
    cached: Mutex<Option<Cached>>,
    counts: Mutex<Option<Counts>>,
    /// The last index that verified, as downloaded.
    saved: PathBuf,
}

impl fmt::Debug for Catalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Catalog")
            .field("channel", &self.channel)
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl Catalog {
    /// Creates a catalog and cleans up after any interrupted install.
    ///
    /// Args:
    ///     root: The user's presences folder, where presences are installed.
    ///     data_dir: Keeps the newest accepted index time and the last
    ///         verified index.
    ///     seal: Seals what is installed, so changes by hand are noticed.
    pub fn new(
        source: Box<dyn Source>,
        keys: Keys,
        channel: Channel,
        root: &Path,
        data_dir: &Path,
        seal: Key,
    ) -> Self {
        install::clean_leftovers(root);
        Self {
            source,
            keys,
            channel,
            root: root.to_owned(),
            history: data_dir.join("presence-history"),
            key: seal,
            seen: Seen::new(data_dir),
            cached: Mutex::new(None),
            counts: Mutex::new(None),
            saved: data_dir.join(format!("{}.json", channel.branch())),
        }
    }

    /// The key that seals installed presences.
    pub fn key(&self) -> &Key {
        &self.key
    }

    /// Why a presence folder must not load, if any.
    pub fn refusal(&self, folder: &Path) -> Option<String> {
        modified_message(&self.installed_at(folder)?)
    }

    /// The record of a folder the catalog installed, with its standing.
    ///
    /// Returns:
    ///     `None` for a folder outside the presences folder, without a marker,
    ///     or whose marker names another folder: those are the user's own.
    pub fn installed_at(&self, folder: &Path) -> Option<Installed> {
        if folder.parent() != Some(self.root.as_path()) {
            return None;
        }
        install::inspect(folder, &self.key)
    }

    /// Tells whether the application carries a key, so the catalog can be used.
    pub fn enabled(&self) -> bool {
        !self.keys.is_empty()
    }

    /// The verified index, downloaded when the copy is old or `refresh` asks.
    ///
    /// Raises:
    ///     Error: It cannot be downloaded, is not signed by a trusted key,
    ///         is malformed, or is older than one accepted before.
    pub fn index(&self, refresh: bool) -> Result<Index, Error> {
        if !self.enabled() {
            return Err(Error::Disabled);
        }
        if !refresh
            && let Some(cached) = self
                .cached
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            && cached.at.elapsed() < CACHE
        {
            return Ok(cached.index.clone());
        }
        // Only a copy with a tag can be confirmed by the server instead of downloaded.
        let kept = self
            .cached
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|cached| {
                let tag = cached.tag.clone()?;
                Some((cached.index.clone(), tag))
            });
        let known = kept.as_ref().map(|(_, tag)| tag.as_str());
        // The download runs without the lock, so a slow network never blocks other callers.
        let answer = self.source.index_if_changed(self.channel, known);
        let (index, tag) = match (answer, kept) {
            // The copy in memory was verified when it came; the server says it is current.
            (Ok(Fetched::Unchanged), Some((index, tag))) => (index, Some(tag)),
            (Ok(Fetched::Unchanged), None) => {
                return Err(Error::Network("an unchanged answer to no copy".into()));
            }
            (Ok(Fetched::New { bytes, tag }), _) => {
                let index = Index::verify(&bytes, &self.keys, self.channel)?;
                self.seen.accept(self.channel, index.generated_at)?;
                self.save(&bytes);
                (index, tag)
            }
            // Without GitHub, the last verified copy still lists the catalog.
            // It is checked again as if it had just been downloaded.
            (Err(error @ (Error::Network(_) | Error::RateLimited)), _) => {
                (self.saved_index().ok_or(error)?, None)
            }
            (Err(error), _) => return Err(error),
        };
        *self.cached.lock().unwrap_or_else(|e| e.into_inner()) = Some(Cached {
            at: Instant::now(),
            index: index.clone(),
            tag,
        });
        Ok(index)
    }

    /// Keeps the bytes of an index that verified, replacing the old copy in one step.
    fn save(&self, bytes: &[u8]) {
        let temporary = self.saved.with_extension("json.tmp");
        if std::fs::write(&temporary, bytes).is_ok()
            && std::fs::rename(&temporary, &self.saved).is_err()
        {
            let _ = std::fs::remove_file(&temporary);
        }
    }

    /// The saved index, if it is still signed by a trusted key, for this
    /// channel and not older than the newest one accepted.
    fn saved_index(&self) -> Option<Index> {
        let size = std::fs::metadata(&self.saved).ok()?.len();
        if size > crate::index::MAX_INDEX {
            return None;
        }
        let bytes = std::fs::read(&self.saved).ok()?;
        let index = Index::verify(&bytes, &self.keys, self.channel).ok()?;
        self.seen.accept(self.channel, index.generated_at).ok()?;
        Some(index)
    }

    /// Lists the catalog with what is installed here.
    ///
    /// Raises:
    ///     Error: As [`Catalog::index`].
    pub fn listing(&self, refresh: bool) -> Result<Vec<Listing>, Error> {
        let index = self.index(refresh)?;
        let here = install::installed(&self.root, &self.key);
        Ok(index
            .presences
            .into_iter()
            .map(|entry| {
                let state = here
                    .iter()
                    .find(|record| {
                        record.folder == entry.folder
                            && record.name.eq_ignore_ascii_case(&entry.name)
                    })
                    .map_or(State::Available, |record| state_of(record, &entry));
                let history = match state {
                    State::Available => Vec::new(),
                    _ => history::versions(&self.history, &entry.folder),
                };
                Listing {
                    entry,
                    state,
                    history,
                }
            })
            .collect())
    }

    /// How many times each listed presence was installed, by name.
    ///
    /// The counts come unsigned and are for display only. They are asked for
    /// at most once an hour, and a presence without one is left out.
    ///
    /// Returns:
    ///     Empty when the catalog or the counts cannot be had.
    pub fn installs(&self) -> BTreeMap<String, u64> {
        let Ok(index) = self.index(false) else {
            return BTreeMap::new();
        };
        let by_folder = self.counts_at(Instant::now());
        index
            .presences
            .into_iter()
            .filter_map(|entry| Some((entry.name, *by_folder.get(&entry.folder)?)))
            .collect()
    }

    /// The counts by folder, downloaded when the copy is old at `now`.
    fn counts_at(&self, now: Instant) -> BTreeMap<String, u64> {
        let kept = {
            let counts = self.counts.lock().unwrap_or_else(|e| e.into_inner());
            match counts.as_ref() {
                Some(kept) if now < kept.until => return kept.by_folder.clone(),
                Some(kept) => Some((kept.by_folder.clone(), kept.tag.clone())),
                None => None,
            }
        };
        let known = kept.as_ref().and_then(|(_, tag)| tag.as_deref());
        // The download runs without the lock, like the one of the index.
        let answer = self.source.installs(known);
        let (by_folder, tag, wait) = match (answer, kept) {
            (Ok(Fetched::New { bytes, tag }), _) => {
                (installs::parse(&bytes, self.channel), tag, COUNTS_CACHE)
            }
            (Ok(Fetched::Unchanged), Some((by_folder, tag))) => (by_folder, tag, COUNTS_CACHE),
            // No answer: what was known stays on display until the next try.
            (_, Some((by_folder, tag))) => (by_folder, tag, COUNTS_RETRY),
            (_, None) => (BTreeMap::new(), None, COUNTS_RETRY),
        };
        *self.counts.lock().unwrap_or_else(|e| e.into_inner()) = Some(Counts {
            until: now + wait,
            by_folder: by_folder.clone(),
            tag,
        });
        by_folder
    }

    /// Installs a presence, or updates it to the version in the catalog, and
    /// returns the entry installed.
    ///
    /// Args:
    ///     accepted: New access the person said yes to; empty the first time.
    ///
    /// Raises:
    ///     Error: `Unknown` if the catalog lacks it, plus anything
    ///         [`install::install`] or [`Catalog::index`] can raise.
    pub fn install(&self, name: &str, accepted: &[Grant]) -> Result<Entry, Error> {
        let index = self.index(false)?;
        let entry = index
            .find(name)
            .ok_or_else(|| Error::Unknown(name.to_owned()))?
            .clone();
        // Refused before downloading: its manifest would be refused after it.
        if entry.too_new() {
            return Err(Error::Newer(entry.name));
        }
        install::install(
            &self.root,
            &self.history,
            &self.key,
            &index,
            &entry,
            self.source.as_ref(),
            accepted,
        )?;
        Ok(entry)
    }

    /// The versions kept from before the last updates, newest first.
    ///
    /// Raises:
    ///     Error: `NotManaged` if the catalog did not install the presence.
    pub fn history(&self, name: &str) -> Result<Vec<Version>, Error> {
        let record = self.managed(name)?;
        Ok(history::versions(&self.history, &record.folder))
    }

    /// Brings back a kept version; the one it replaces is kept in turn.
    ///
    /// Raises:
    ///     Error: `NotManaged` if the catalog did not install the presence,
    ///         `Unknown` if that version is not kept.
    pub fn restore(&self, name: &str, version: &Version) -> Result<Installed, Error> {
        let record = self.managed(name)?;
        install::restore(&self.root, &self.history, &self.key, &record, version)
    }

    fn managed(&self, name: &str) -> Result<Installed, Error> {
        install::installed(&self.root, &self.key)
            .into_iter()
            .find(|record| record.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| Error::NotManaged(name.to_owned()))
    }

    /// Removes a presence that the catalog installed.
    ///
    /// Returns:
    ///     The name it was installed under, which may differ from `name` in case.
    ///
    /// Raises:
    ///     Error: `NotManaged` if it was not installed from the catalog.
    pub fn remove(&self, name: &str) -> Result<String, Error> {
        let record = install::remove(&self.root, &self.key, name)?;
        // A presence that is gone has nothing to go back to.
        history::clear(&self.history, &record.folder);
        Ok(record.name)
    }
}

/// What to tell the user about a catalog folder changed by hand; `None` while
/// it is as installed.
pub fn modified_message(record: &Installed) -> Option<String> {
    (record.standing == Standing::Modified).then(|| {
        format!(
            "{} was changed after it was installed from the catalog; \
             reinstall it, or copy it to edit it",
            record.name
        )
    })
}

fn state_of(record: &Installed, entry: &Entry) -> State {
    if record.standing == Standing::Modified {
        State::Modified {
            installed: record.version.clone(),
        }
    } else if entry.version > record.version && !entry.too_new() {
        State::Update {
            installed: record.version.clone(),
        }
    } else {
        State::Installed {
            version: record.version.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
