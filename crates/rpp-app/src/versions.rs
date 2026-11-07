//! Saved versions of a presence the user wrote, without needing Git.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::files::{Files, read_folder, safe_path, sha256};
use crate::views::{Change, FileChange, VersionView};

/// Versions kept for each presence.
const MAX_VERSIONS: usize = 30;
/// Longest note, in characters.
const MAX_NOTES: usize = 500;
/// Largest record read; a real one lists at most a few hundred bytes per file.
const MAX_RECORD: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    sha256: String,
    size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    id: u32,
    at: u64,
    version: String,
    notes: String,
    files: BTreeMap<String, Stored>,
}

impl Record {
    fn view(&self) -> VersionView {
        VersionView {
            id: self.id,
            at: self.at,
            version: self.version.clone(),
            notes: self.notes.clone(),
            files: self.files.len(),
            size: self.files.values().map(|file| file.size).sum(),
        }
    }
}

/// Replaces a file whole, so a crash never leaves half of it.
fn write_whole(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    // A new file, so a link left under this name is not followed.
    let _ = fs::remove_file(&temporary);
    let written = File::create_new(&temporary).and_then(|mut file| file.write_all(bytes));
    written.map_err(|e| e.to_string())?;
    fs::rename(&temporary, path).map_err(|e| e.to_string())
}

fn inventory(files: &Files) -> BTreeMap<String, Stored> {
    files
        .iter()
        .map(|(path, bytes)| {
            let stored = Stored {
                sha256: sha256(bytes),
                size: bytes.len() as u64,
            };
            (path.clone(), stored)
        })
        .collect()
}

/// Every readable record, oldest first; one that does not parse is skipped.
fn records(dir: &Path) -> Vec<Record> {
    let Ok(entries) = fs::read_dir(dir.join("versions")) else {
        return Vec::new();
    };
    let mut all: Vec<Record> = entries
        .flatten()
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.len() <= MAX_RECORD))
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
        .collect();
    all.sort_by_key(|record| record.id);
    all
}

fn record_path(dir: &Path, id: u32) -> PathBuf {
    dir.join("versions").join(format!("{id}.json"))
}

/// Saves the folder as a new version, unless it matches the latest one.
fn store(
    dir: &Path,
    files: &Files,
    version: &str,
    notes: &str,
    at: u64,
) -> Result<Option<Record>, String> {
    let listed = inventory(files);
    let existing = records(dir);
    if existing.last().is_some_and(|last| last.files == listed) {
        return Ok(None);
    }
    for (path, stored) in &listed {
        let blob = dir.join("blobs").join(&stored.sha256);
        if !blob.exists() {
            write_whole(&blob, &files[path])?;
        }
    }
    let record = Record {
        id: existing.last().map_or(1, |last| last.id.saturating_add(1)),
        at,
        version: version.to_owned(),
        notes: notes.trim().chars().take(MAX_NOTES).collect(),
        files: listed,
    };
    let text = serde_json::to_vec_pretty(&record).map_err(|e| e.to_string())?;
    write_whole(&record_path(dir, record.id), &text)?;
    prune(dir);
    Ok(Some(record))
}

/// Drops the oldest records past the limit and the files nothing uses now.
fn prune(dir: &Path) {
    let all = records(dir);
    if all.len() <= MAX_VERSIONS {
        return;
    }
    let (old, kept) = all.split_at(all.len() - MAX_VERSIONS);
    for record in old {
        let _ = fs::remove_file(record_path(dir, record.id));
    }
    let used: BTreeSet<&str> = kept
        .iter()
        .flat_map(|record| record.files.values())
        .map(|file| file.sha256.as_str())
        .collect();
    let Ok(blobs) = fs::read_dir(dir.join("blobs")) else {
        return;
    };
    for blob in blobs.flatten() {
        if !used.contains(blob.file_name().to_string_lossy().as_ref()) {
            let _ = fs::remove_file(blob.path());
        }
    }
}

/// Saves a presence folder as a new version.
///
/// Args:
///     dir: Where this presence keeps its versions.
///     version: The version its manifest declares.
///     at: Unix seconds.
///
/// Raises:
///     String: Nothing changed since the last version, or a file cannot be
///         read or written.
pub(crate) fn save(
    dir: &Path,
    folder: &Path,
    version: &str,
    notes: &str,
    at: u64,
) -> Result<VersionView, String> {
    let files = read_folder(folder)?;
    store(dir, &files, version, notes, at)?
        .map(|record| record.view())
        .ok_or_else(|| "Nothing changed since the last version.".to_owned())
}

/// The saved versions, newest first.
pub(crate) fn list(dir: &Path) -> Vec<VersionView> {
    records(dir).iter().rev().map(Record::view).collect()
}

fn find(dir: &Path, id: u32) -> Result<Record, String> {
    records(dir)
        .into_iter()
        .find(|record| record.id == id)
        .ok_or_else(|| format!("Version {id} is not saved."))
}

/// Reads a version's files, refusing any that are not what the record says.
fn load(dir: &Path, record: &Record) -> Result<Files, String> {
    let mut files = Files::new();
    for (path, stored) in &record.files {
        if !safe_path(path) {
            return Err(format!("Version {} has an unsafe path.", record.id));
        }
        let digest =
            stored.sha256.len() == 64 && stored.sha256.bytes().all(|byte| byte.is_ascii_hexdigit());
        let bytes = digest
            .then(|| fs::read(dir.join("blobs").join(&stored.sha256)).ok())
            .flatten()
            .filter(|bytes| sha256(bytes) == stored.sha256)
            .ok_or_else(|| format!("{path} of version {} is damaged.", record.id))?;
        files.insert(path.clone(), bytes);
    }
    Ok(files)
}

/// Counts lines added and removed, ignoring their order.
fn line_changes(before: &[u8], after: &[u8]) -> (usize, usize) {
    let (Ok(before), Ok(after)) = (std::str::from_utf8(before), std::str::from_utf8(after)) else {
        return (0, 0);
    };
    let mut balance: HashMap<&str, i64> = HashMap::new();
    for line in after.lines() {
        *balance.entry(line).or_default() += 1;
    }
    for line in before.lines() {
        *balance.entry(line).or_default() -= 1;
    }
    let total = |sign: i64| {
        balance
            .values()
            .filter(|count| count.signum() == sign)
            .map(|count| count.unsigned_abs() as usize)
            .sum()
    };
    (total(1), total(-1))
}

/// What changed by path between a saved version and the folder as it is now.
///
/// Raises:
///     String: The version is not saved, is damaged, or the folder cannot be read.
pub(crate) fn compare(dir: &Path, id: u32, folder: &Path) -> Result<Vec<FileChange>, String> {
    let saved = load(dir, &find(dir, id)?)?;
    let current = read_folder(folder)?;
    let paths: BTreeSet<&String> = saved.keys().chain(current.keys()).collect();
    let mut changes = Vec::new();
    for path in paths {
        let (before, after) = (saved.get(path), current.get(path));
        let change = match (before, after) {
            (Some(a), Some(b)) if a == b => continue,
            (None, _) => Change::Added,
            (_, None) => Change::Removed,
            _ => Change::Changed,
        };
        let empty = Vec::new();
        let (added, removed) = line_changes(before.unwrap_or(&empty), after.unwrap_or(&empty));
        changes.push(FileChange {
            path: path.clone(),
            change,
            added,
            removed,
        });
    }
    Ok(changes)
}

/// Puts a saved version back in the folder.
///
/// Args:
///     version: The version the manifest declares now.
///     at: Unix seconds.
///
/// Raises:
///     String: The version is not saved or is damaged, or a file cannot be written.
pub(crate) fn restore(
    dir: &Path,
    id: u32,
    folder: &Path,
    version: &str,
    at: u64,
) -> Result<(), String> {
    let record = find(dir, id)?;
    let saved = load(dir, &record)?;
    let current = read_folder(folder)?;
    store(
        dir,
        &current,
        version,
        &format!("Before restoring version {id}"),
        at,
    )?;
    for (path, bytes) in &saved {
        if current.get(path) != Some(bytes) {
            write_whole(&folder.join(path), bytes)?;
        }
    }
    for path in current.keys().filter(|path| !saved.contains_key(*path)) {
        fs::remove_file(folder.join(path)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Bench {
        store: tempfile::TempDir,
        folder: tempfile::TempDir,
    }

    impl Bench {
        fn new() -> Self {
            let bench = Self {
                store: tempfile::tempdir().unwrap(),
                folder: tempfile::tempdir().unwrap(),
            };
            bench.write("main.js", "line one\nline two\n");
            bench.write("logo.png", "PICTURE");
            bench
        }

        fn write(&self, path: &str, text: &str) {
            let path = self.folder.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }

        fn read(&self, path: &str) -> String {
            fs::read_to_string(self.folder.path().join(path)).unwrap()
        }

        fn save(&self, notes: &str) -> Result<VersionView, String> {
            save(self.store.path(), self.folder.path(), "1.0.0", notes, 7)
        }

        fn blobs(&self) -> usize {
            fs::read_dir(self.store.path().join("blobs"))
                .unwrap()
                .count()
        }
    }

    #[test]
    fn saving_records_the_version_and_lists_the_newest_first() {
        let bench = Bench::new();
        let first = bench.save("  first  ").unwrap();
        assert_eq!((first.id, first.notes.as_str()), (1, "first"));
        assert_eq!((first.files, first.size), (2, 25));
        bench.write("main.js", "changed\n");
        let second = bench.save("second").unwrap();
        assert_eq!(second.id, 2);
        let listed = list(bench.store.path());
        assert_eq!(listed, [second, first]);
    }

    #[test]
    fn an_unchanged_folder_is_not_saved_twice() {
        let bench = Bench::new();
        bench.save("").unwrap();
        assert_eq!(
            bench.save("").unwrap_err(),
            "Nothing changed since the last version."
        );
        assert_eq!(list(bench.store.path()).len(), 1);
    }

    #[test]
    fn a_file_shared_by_versions_is_kept_once() {
        let bench = Bench::new();
        bench.save("").unwrap();
        assert_eq!(bench.blobs(), 2);
        bench.write("main.js", "changed\n");
        bench.save("").unwrap();
        // The logo did not change; only the script adds a file.
        assert_eq!(bench.blobs(), 3);
    }

    #[test]
    fn old_versions_go_and_take_the_files_only_they_used() {
        let bench = Bench::new();
        for number in 0..MAX_VERSIONS + 5 {
            bench.write("main.js", &format!("version {number}\n"));
            bench.save("").unwrap();
        }
        let listed = list(bench.store.path());
        assert_eq!(listed.len(), MAX_VERSIONS);
        assert_eq!(listed.last().unwrap().id, 6);
        assert_eq!(bench.blobs(), MAX_VERSIONS + 1);
        // Ids keep growing, so a number is never used for two versions.
        bench.write("main.js", "one more\n");
        assert_eq!(bench.save("").unwrap().id as usize, MAX_VERSIONS + 6);
    }

    #[test]
    fn comparing_names_each_change_with_its_lines() {
        let bench = Bench::new();
        let id = bench.save("").unwrap().id;
        bench.write("main.js", "line one\nline 2\nline 3\n");
        bench.write("extra.js", "new\n");
        fs::remove_file(bench.folder.path().join("logo.png")).unwrap();
        let changes = compare(bench.store.path(), id, bench.folder.path()).unwrap();
        let summary: Vec<(&str, Change, usize, usize)> = changes
            .iter()
            .map(|c| (c.path.as_str(), c.change, c.added, c.removed))
            .collect();
        assert_eq!(
            summary,
            [
                ("extra.js", Change::Added, 1, 0),
                ("logo.png", Change::Removed, 0, 1),
                ("main.js", Change::Changed, 2, 1),
            ]
        );
    }

    #[test]
    fn restoring_brings_back_files_removes_new_ones_and_can_be_undone() {
        let bench = Bench::new();
        let id = bench.save("").unwrap().id;
        bench.write("main.js", "rewritten\n");
        bench.write("new.js", "new\n");
        fs::remove_file(bench.folder.path().join("logo.png")).unwrap();

        restore(bench.store.path(), id, bench.folder.path(), "2.0.0", 9).unwrap();
        assert_eq!(bench.read("main.js"), "line one\nline two\n");
        assert_eq!(bench.read("logo.png"), "PICTURE");
        assert!(!bench.folder.path().join("new.js").exists());

        // What was there before the restore is a version of its own.
        let listed = list(bench.store.path());
        assert_eq!(listed[0].notes, "Before restoring version 1");
        assert_eq!(listed[0].version, "2.0.0");
        restore(
            bench.store.path(),
            listed[0].id,
            bench.folder.path(),
            "2.0.0",
            10,
        )
        .unwrap();
        assert_eq!(bench.read("main.js"), "rewritten\n");
        assert!(bench.folder.path().join("new.js").exists());
    }

    #[test]
    fn restoring_keeps_hidden_files() {
        let bench = Bench::new();
        let id = bench.save("").unwrap().id;
        bench.write(".git/config", "keep me");
        bench.write("main.js", "changed\n");
        restore(bench.store.path(), id, bench.folder.path(), "1", 8).unwrap();
        assert_eq!(bench.read(".git/config"), "keep me");
    }

    #[test]
    fn a_damaged_version_changes_nothing() {
        let bench = Bench::new();
        let id = bench.save("").unwrap().id;
        let record = find(bench.store.path(), id).unwrap();
        let blob = record.files["main.js"].sha256.clone();
        fs::write(bench.store.path().join("blobs").join(blob), "tampered").unwrap();
        bench.write("main.js", "my work\n");

        let error = restore(bench.store.path(), id, bench.folder.path(), "1", 8).unwrap_err();
        assert!(error.contains("damaged"), "{error}");
        assert_eq!(bench.read("main.js"), "my work\n");
        assert_eq!(list(bench.store.path()).len(), 1);
    }

    #[test]
    fn a_record_edited_to_leave_the_folder_is_refused() {
        let bench = Bench::new();
        let id = bench.save("").unwrap().id;
        let path = record_path(bench.store.path(), id);
        let edited = fs::read_to_string(&path)
            .unwrap()
            .replace("\"main.js\"", "\"../outside.js\"");
        fs::write(&path, edited).unwrap();
        let error = restore(bench.store.path(), id, bench.folder.path(), "1", 8).unwrap_err();
        assert!(error.contains("unsafe"), "{error}");
        assert!(
            !bench
                .folder
                .path()
                .parent()
                .unwrap()
                .join("outside.js")
                .exists()
        );
    }

    #[test]
    fn notes_are_cut_and_unreadable_records_are_skipped() {
        let bench = Bench::new();
        let long = "x".repeat(MAX_NOTES + 50);
        assert_eq!(bench.save(&long).unwrap().notes.len(), MAX_NOTES);
        fs::write(record_path(bench.store.path(), 99), "not json").unwrap();
        assert_eq!(list(bench.store.path()).len(), 1);
    }
}
