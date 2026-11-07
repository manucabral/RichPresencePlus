//! Old copies of installed presences, kept so an update can be undone.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use semver::Version;

/// Copies kept for each presence.
pub const KEPT: usize = 2;

/// Where the copy of a version lives.
pub(crate) fn slot(history: &Path, folder: &str, version: &Version) -> PathBuf {
    history.join(folder).join(version.to_string())
}

/// The versions kept for a presence, newest first.
pub fn versions(history: &Path, folder: &str) -> Vec<Version> {
    let Ok(entries) = fs::read_dir(history.join(folder)) else {
        return Vec::new();
    };
    let mut found: Vec<Version> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
        .collect();
    found.sort_by(|a, b| b.cmp(a));
    found
}

/// Moves an old copy into the history and drops the oldest ones past [`KEPT`].
///
/// Raises:
///     io::Error: The copy cannot be moved; it is left where it was.
pub fn keep(history: &Path, folder: &str, version: &Version, copy: &Path) -> io::Result<()> {
    let target = slot(history, folder, version);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    fs::rename(copy, &target)?;
    for old in versions(history, folder).into_iter().skip(KEPT) {
        let _ = fs::remove_dir_all(slot(history, folder, &old));
    }
    Ok(())
}

/// Deletes every kept copy of a presence.
pub fn clear(history: &Path, folder: &str) {
    let _ = fs::remove_dir_all(history.join(folder));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy(root: &Path, name: &str) -> PathBuf {
        let path = root.join(name);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("main.js"), name).unwrap();
        path
    }

    #[test]
    fn only_the_newest_copies_are_kept_and_they_list_newest_first() {
        let history = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        for (index, version) in ["1.0.0", "1.2.0", "1.1.0"].iter().enumerate() {
            let parsed: Version = version.parse().unwrap();
            keep(
                history.path(),
                "demo",
                &parsed,
                &copy(work.path(), &format!("c{index}")),
            )
            .unwrap();
        }
        let kept = versions(history.path(), "demo");
        assert_eq!(
            kept,
            [
                "1.2.0".parse::<Version>().unwrap(),
                "1.1.0".parse().unwrap()
            ]
        );
        assert!(!slot(history.path(), "demo", &"1.0.0".parse().unwrap()).exists());
    }

    #[test]
    fn keeping_the_same_version_again_replaces_it() {
        let history = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let version: Version = "1.0.0".parse().unwrap();
        keep(
            history.path(),
            "demo",
            &version,
            &copy(work.path(), "first"),
        )
        .unwrap();
        keep(
            history.path(),
            "demo",
            &version,
            &copy(work.path(), "second"),
        )
        .unwrap();
        let text =
            fs::read_to_string(slot(history.path(), "demo", &version).join("main.js")).unwrap();
        assert_eq!(text, "second");
        assert_eq!(versions(history.path(), "demo").len(), 1);
    }

    #[test]
    fn clearing_removes_every_kept_copy() {
        let history = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let version: Version = "1.0.0".parse().unwrap();
        keep(history.path(), "demo", &version, &copy(work.path(), "c")).unwrap();
        clear(history.path(), "demo");
        assert!(versions(history.path(), "demo").is_empty());
        clear(history.path(), "demo");
    }

    #[test]
    fn a_missing_history_lists_nothing_and_foreign_names_are_ignored() {
        let history = tempfile::tempdir().unwrap();
        assert!(versions(history.path(), "demo").is_empty());
        fs::create_dir_all(history.path().join("demo").join("not-a-version")).unwrap();
        fs::write(
            history.path().join("demo").join("1.0.0"),
            "a file, not a copy",
        )
        .unwrap();
        assert!(versions(history.path(), "demo").is_empty());
    }
}
