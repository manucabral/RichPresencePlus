//! Putting a presence on disk, and taking it off.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rpp_plugins::{Grant, Manifest};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::bundle;
use crate::error::Error;
use crate::history;
use crate::index::{Entry, Index, plain_folder};
use crate::seal::{self, Key};
use crate::source::Source;

/// Name of the file that marks a folder as installed from the catalog.
const MARKER: &str = ".catalog.json";
/// Largest marker file read: room for a record of every file the index allows.
const MAX_MARKER: u64 = 64 * 1024;

/// Whether an installed folder is still what the catalog put there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Standing {
    /// The seal verifies and every file is the one that was installed.
    Intact,
    /// A file changed, was added or removed, or the seal is not valid.
    #[default]
    Modified,
}

/// What the marker file records about an installed presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Installed {
    /// Name of the presence.
    pub name: String,
    /// Folder it lives in.
    pub folder: String,
    /// Version that was installed.
    pub version: Version,
    /// Commit its files came from.
    pub commit: String,
    /// SHA-256 of every file that was installed, by path.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// HMAC over the fields above, made with this installation's key.
    #[serde(default)]
    pub seal: String,
    /// Whether the folder still matches; worked out when the marker is read.
    #[serde(skip)]
    pub standing: Standing,
}

/// Lower-case hexadecimal SHA-256.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A staging folder that is removed unless it was moved into place.
struct Staging(Option<PathBuf>);

impl Staging {
    fn create(root: &Path, folder: &str) -> Result<Self, Error> {
        let path = root.join(format!(".installing-{folder}"));
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        fs::create_dir_all(&path)?;
        Ok(Self(Some(path)))
    }

    fn path(&self) -> &Path {
        self.0.as_deref().unwrap_or_else(|| Path::new(""))
    }

    /// Gives the folder up so it is not removed.
    fn release(mut self) -> PathBuf {
        self.0.take().unwrap_or_default()
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = fs::remove_dir_all(path);
        }
    }
}

/// Checks that the downloaded manifest is the one the index announced.
fn check_manifest(folder: &Path, entry: &Entry) -> Result<Manifest, Error> {
    let text = fs::read_to_string(folder.join("manifest.json"))?;
    let manifest =
        Manifest::parse(&text).map_err(|e| Error::Invalid(format!("{}: {e}", entry.name)))?;
    let version = Version::parse(&manifest.version).ok();
    if manifest.name != entry.name || version.as_ref() != Some(&entry.version) {
        return Err(Error::Invalid(format!(
            "{}: the manifest does not match the index",
            entry.name
        )));
    }
    // The script has to be one of the signed files, not something the manifest points at.
    if !entry.files.contains_key(&manifest.entry) {
        return Err(Error::Invalid(format!(
            "{}: the script is not among the signed files",
            entry.name
        )));
    }
    Ok(manifest)
}

/// Stops an update that reaches for more than the copy it replaces.
///
/// Raises:
///     Error: `NewAccess` with what the update adds and nobody accepted.
fn check_access(installed: &Path, update: &Manifest, accepted: &[Grant]) -> Result<(), Error> {
    if !installed.exists() {
        return Ok(());
    }
    let before = fs::read_to_string(installed.join("manifest.json"))
        .ok()
        .and_then(|text| Manifest::parse(&text).ok())
        .map(|manifest| manifest.grants())
        .unwrap_or_default();
    let new: Vec<Grant> = update
        .grants()
        .into_iter()
        .filter(|grant| !before.contains(grant) && !accepted.contains(grant))
        .collect();
    if new.is_empty() {
        Ok(())
    } else {
        Err(Error::NewAccess(new))
    }
}

/// Tells whether a folder is what its marker says the catalog installed.
fn verify(folder: &Path, record: &Installed, key: &Key) -> Standing {
    if seal::verifies(key, record) && seal::scan(folder).as_ref() == Some(&record.files) {
        Standing::Intact
    } else {
        Standing::Modified
    }
}

/// Moves a replaced copy into the history.
fn archive(history: &Path, folder: &str, previous: &Path, key: &Key) {
    let kept = read_marker(previous)
        .filter(|record| {
            record.folder == folder && verify(previous, record, key) == Standing::Intact
        })
        .map(|record| history::keep(history, folder, &record.version, previous));
    if !matches!(kept, Some(Ok(()))) {
        let _ = fs::remove_dir_all(previous);
    }
}

/// Stops an install that would replace a folder the catalog did not write.
///
/// Raises:
///     Error: The folder exists and has no marker of the catalog.
fn refuse_a_folder_of_the_user(path: &Path, folder: &str) -> Result<(), Error> {
    if path.exists() && marker(path).is_none() {
        return Err(Error::Invalid(format!(
            "{folder}: a folder you made already uses this name"
        )));
    }
    Ok(())
}

/// Replaces `root/folder` with the staged copy.
fn swap(
    root: &Path,
    history: &Path,
    key: &Key,
    staged: Staging,
    folder: &str,
) -> Result<(), Error> {
    let target = root.join(folder);
    // The download took a while, and the user can create a presence meanwhile.
    refuse_a_folder_of_the_user(&target, folder)?;
    let previous = root.join(format!(".previous-{folder}"));
    if previous.exists() {
        fs::remove_dir_all(&previous)?;
    }
    let had_old = target.exists();
    if had_old {
        fs::rename(&target, &previous)?;
    }
    let source = staged.release();
    if let Err(error) = fs::rename(&source, &target) {
        // Put the old copy back, then report; the staging folder goes with it.
        if had_old {
            let _ = fs::rename(&previous, &target);
        }
        let _ = fs::remove_dir_all(&source);
        return Err(error.into());
    }
    if had_old {
        archive(history, folder, &previous, key);
    }
    Ok(())
}

/// Downloads every file of an entry and checks each against the index.
fn download(
    index: &Index,
    entry: &Entry,
    source: &dyn Source,
) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    if let Some(name) = &entry.bundle {
        let opened = source
            .bundle(index.channel.release(), name, bundle::limit(entry))
            .and_then(|bytes| bundle::unpack(&bytes, entry));
        if let Ok(files) = opened {
            return Ok(files);
        }
    }
    let mut files = BTreeMap::new();
    for (path, info) in &entry.files {
        let bytes = source.file(&index.commit, &entry.folder, path, info.size)?;
        if bytes.len() as u64 != info.size || sha256_hex(&bytes) != info.sha256 {
            return Err(Error::Integrity(format!("{}/{path}", entry.folder)));
        }
        files.insert(path.clone(), bytes);
    }
    Ok(files)
}

/// Downloads, verifies and installs one presence.
///
/// Args:
///     history: Where the copy being replaced is kept.
///     accepted: New access the person said yes to; empty the first time.
///
/// Raises:
///     Error: `Network` if a download fails, `Integrity` if a file differs
///         from the index, `Invalid` if the manifest does not match or the
///         folder is not the catalog's, `NewAccess` if the update asks for more
///         than the installed copy, `Disk` if the folder cannot be written.
pub fn install(
    root: &Path,
    history: &Path,
    key: &Key,
    index: &Index,
    entry: &Entry,
    source: &dyn Source,
    accepted: &[Grant],
) -> Result<(), Error> {
    if !plain_folder(&entry.folder) {
        return Err(Error::Invalid("the folder name is not plain".into()));
    }
    fs::create_dir_all(root)?;
    refuse_a_folder_of_the_user(&root.join(&entry.folder), &entry.folder)?;
    let contents = download(index, entry, source)?;
    let staged = Staging::create(root, &entry.folder)?;
    for (path, bytes) in contents {
        let target = staged.path().join(&path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, bytes)?;
    }
    let manifest = check_manifest(staged.path(), entry)?;
    check_access(&root.join(&entry.folder), &manifest, accepted)?;
    let mut record = Installed {
        name: entry.name.clone(),
        folder: entry.folder.clone(),
        version: entry.version.clone(),
        commit: index.commit.clone(),
        files: entry
            .files
            .iter()
            .map(|(path, info)| (path.clone(), info.sha256.clone()))
            .collect(),
        seal: String::new(),
        standing: Standing::Intact,
    };
    // What is on disk must be exactly what the seal will cover.
    if seal::scan(staged.path()).as_ref() != Some(&record.files) {
        return Err(Error::Invalid(format!(
            "{}: the files on disk differ from the index",
            entry.name
        )));
    }
    record.seal = seal::sign(key, &record);
    let text = serde_json::to_string(&record).map_err(|e| Error::Disk(e.to_string()))?;
    fs::write(staged.path().join(MARKER), text)?;
    swap(root, history, key, staged, &entry.folder)
}

/// Reads the marker file of a folder, wherever the folder is.
fn read_marker(folder: &Path) -> Option<Installed> {
    let path = folder.join(MARKER);
    if fs::metadata(&path).ok()?.len() > MAX_MARKER {
        return None;
    }
    let found: Installed = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    plain_folder(&found.folder).then_some(found)
}

/// Reads the marker of a presence folder, if it has one that names that folder.
fn marker(folder: &Path) -> Option<Installed> {
    let found = read_marker(folder)?;
    // The marker names the folder it sits in; a copied folder is not trusted.
    (folder.file_name()?.to_str()? == found.folder).then_some(found)
}

/// Reads a presence folder's marker and checks the folder against it.
///
/// Returns:
///     The record with its standing, or `None` if the folder has no marker or
///     the marker names another folder, which makes it a folder the user made.
pub fn inspect(folder: &Path, key: &Key) -> Option<Installed> {
    let mut record = marker(folder)?;
    record.standing = verify(folder, &record, key);
    Some(record)
}

/// One record per presence installed from the catalog, sorted by name. A
/// folder changed after the install is listed as `Modified`.
pub fn installed(root: &Path, key: &Key) -> Vec<Installed> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<Installed> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| inspect(&entry.path(), key))
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Removes a presence that was installed from the catalog.
///
/// Raises:
///     Error: `NotManaged` if the catalog did not install it, `Disk` if it cannot be deleted.
pub fn remove(root: &Path, key: &Key, name: &str) -> Result<Installed, Error> {
    let record = installed(root, key)
        .into_iter()
        .find(|record| record.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| Error::NotManaged(name.to_owned()))?;
    let folder = root.join(&record.folder);
    // Moving first makes the removal one step: the presence is gone for the
    // loader even if deleting the files is interrupted.
    let doomed = root.join(format!(".removing-{}", record.folder));
    if doomed.exists() {
        fs::remove_dir_all(&doomed)?;
    }
    fs::rename(&folder, &doomed)?;
    fs::remove_dir_all(&doomed)?;
    Ok(record)
}

/// Puts a kept version back, and keeps the one it replaces.
///
/// Raises:
///     Error: `Unknown` if that version is not kept, `Integrity` if the kept copy
///         was changed, `Disk` if the folders cannot be moved.
pub fn restore(
    root: &Path,
    history: &Path,
    key: &Key,
    record: &Installed,
    version: &semver::Version,
) -> Result<Installed, Error> {
    let folder = record.folder.as_str();
    let copy = history::slot(history, folder, version);
    if !plain_folder(folder) || !copy.is_dir() {
        return Err(Error::Unknown(format!("{folder} {version}")));
    }
    let untouched = read_marker(&copy).is_some_and(|kept| {
        kept.folder == folder
            && kept.version == *version
            && verify(&copy, &kept, key) == Standing::Intact
    });
    if !untouched {
        return Err(Error::Integrity(format!("{folder} {version}")));
    }
    let target = root.join(folder);
    let previous = root.join(format!(".previous-{folder}"));
    if previous.exists() {
        fs::remove_dir_all(&previous)?;
    }
    fs::rename(&target, &previous)?;
    if let Err(error) = fs::rename(&copy, &target) {
        let _ = fs::rename(&previous, &target);
        return Err(error.into());
    }
    archive(history, folder, &previous, key);
    inspect(&target, key)
        .ok_or_else(|| Error::Disk(format!("{folder} has no marker after restoring")))
}

/// Cleans up after an install or removal that was interrupted.
pub fn clean_leftovers(root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if !path.is_dir() {
            continue;
        }
        if let Some(folder) = name.strip_prefix(".previous-") {
            let target = root.join(folder);
            if !target.exists() && plain_folder(folder) && fs::rename(&path, &target).is_ok() {
                continue;
            }
            let _ = fs::remove_dir_all(&path);
        } else if name.starts_with(".installing-") || name.starts_with(".removing-") {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
