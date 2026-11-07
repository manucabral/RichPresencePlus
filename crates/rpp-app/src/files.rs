//! Reading the files of a presence folder, for the developer tools.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::templates::{COPY_BYTES, COPY_FILES};

/// The files of a presence by relative path, with `/` on every system.
pub(crate) type Files = BTreeMap<String, Vec<u8>>;

/// Reads every file of a folder.
///
/// Raises:
///     String: The folder cannot be read or is too large.
pub(crate) fn read_folder(folder: &Path) -> Result<Files, String> {
    let mut files = Files::new();
    let mut bytes = 0;
    walk(folder, "", &mut files, &mut bytes)?;
    Ok(files)
}

fn walk(folder: &Path, prefix: &str, files: &mut Files, bytes: &mut u64) -> Result<(), String> {
    for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if name.starts_with('.') || name == crate::editor::JSCONFIG || kind.is_symlink() {
            continue;
        }
        let path = format!("{prefix}{name}");
        if kind.is_dir() {
            walk(&entry.path(), &format!("{path}/"), files, bytes)?;
        } else if kind.is_file() {
            *bytes += entry.metadata().map_err(|e| e.to_string())?.len();
            if files.len() >= COPY_FILES || *bytes > COPY_BYTES {
                return Err("The presence is too large.".into());
            }
            files.insert(path, fs::read(entry.path()).map_err(|e| e.to_string())?);
        }
    }
    Ok(())
}

/// The SHA-256 of some bytes in lowercase hexadecimal.
pub(crate) fn sha256(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// Tells whether a stored path stays inside the folder it is written to.
pub(crate) fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_files_are_left_out_and_paths_use_slashes() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("locales")).unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join("main.js"), "a").unwrap();
        fs::write(dir.path().join("locales/en.json"), "b").unwrap();
        fs::write(dir.path().join(".env"), "secret").unwrap();
        fs::write(dir.path().join(".git/config"), "c").unwrap();
        let files = read_folder(dir.path()).unwrap();
        let names: Vec<&str> = files.keys().map(String::as_str).collect();
        assert_eq!(names, ["locales/en.json", "main.js"]);
    }

    #[test]
    fn too_many_files_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        for number in 0..=COPY_FILES {
            fs::write(dir.path().join(format!("{number}.txt")), "x").unwrap();
        }
        assert!(read_folder(dir.path()).is_err());
    }

    #[test]
    fn the_hash_is_the_standard_one() {
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn paths_that_leave_the_folder_are_refused() {
        for good in ["main.js", "locales/en.json", "a..b/c"] {
            assert!(safe_path(good), "{good}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "../main.js",
            "a/../../b",
            "a//b",
            "./a",
            r"a\b",
            r"C:\Windows\x",
            "C:evil",
            "a\0b",
        ] {
            assert!(!safe_path(bad), "{bad}");
        }
    }
}
