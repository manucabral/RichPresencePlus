//! Reading the end of a log file.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};

use serde_json::Value;

/// Most bytes read from the end of a file.
const MAX_BYTES: u64 = 256 * 1024;

/// Lowercase, forward slashes and no `\\?\` prefix, so two spellings of the
/// same location compare equal.
fn comparable(path: &str) -> String {
    let text = path.replace('\\', "/").to_lowercase();
    text.strip_prefix("//?/").unwrap_or(&text).to_owned()
}

/// Spells a resolved path the way scripts and permissions write it: Windows
/// adds `\\?\` to resolved paths, which would never match a declared folder.
fn plain(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => rest.to_owned(),
        _ => text.into_owned(),
    }
}

/// Path of the newest file in `dir` whose name ends with `suffix`, such as
/// `_last.log`; `null` when there is none. Paths through links are refused.
pub fn latest(dir: &str, suffix: &str) -> Result<Value, String> {
    let resolved = fs::canonicalize(dir).map_err(|e| format!("cannot open {dir}: {e}"))?;
    if comparable(&resolved.to_string_lossy()) != comparable(dir).trim_end_matches('/') {
        return Err("path must not pass through a link".into());
    }
    let mut newest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for entry in fs::read_dir(&resolved)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let name = entry.file_name();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_file() || !name.to_string_lossy().ends_with(suffix) {
            continue;
        }
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        if newest.as_ref().is_none_or(|(best, _)| modified > *best) {
            newest = Some((modified, entry.path()));
        }
    }
    Ok(newest.map_or(Value::Null, |(_, path)| Value::from(plain(&path))))
}

/// Reads the last lines of a file, at most its final 256 KiB. Paths through
/// links are refused.
pub fn tail(path: &str, lines: usize) -> Result<Value, String> {
    let resolved = fs::canonicalize(path).map_err(|e| format!("cannot open {path}: {e}"))?;
    let resolved = resolved.to_string_lossy();
    if comparable(&resolved) != comparable(path).trim_end_matches('/') {
        return Err("path must not pass through a link".into());
    }
    let mut file = File::open(&*resolved).map_err(|e| e.to_string())?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    let start = size.saturating_sub(MAX_BYTES);
    file.seek(SeekFrom::Start(start))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    let mut all: Vec<&str> = text.lines().collect();
    if start > 0 && !all.is_empty() {
        // The first line starts before the window and is incomplete.
        all.remove(0);
    }
    let from = all.len().saturating_sub(lines);
    Ok(all[from..].iter().map(|line| Value::from(*line)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(dir: &tempfile::TempDir, content: &[u8]) -> String {
        let path = dir.path().join("game.log");
        File::create(&path).unwrap().write_all(content).unwrap();
        // Temp folders can sit behind a link or 8.3 name; use the resolved spelling.
        fs::canonicalize(path)
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn returns_the_last_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(&dir, b"one\ntwo\nthree\nfour\n");
        assert_eq!(
            tail(&path, 2).unwrap(),
            serde_json::json!(["three", "four"])
        );
        assert_eq!(tail(&path, 10).unwrap().as_array().unwrap().len(), 4);
    }

    #[test]
    fn only_the_end_of_a_large_file_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let body: String = (0..100_000).map(|n| format!("line {n}\n")).collect();
        let path = write(&dir, body.as_bytes());
        let lines = tail(&path, 3).unwrap();
        assert_eq!(lines[2], "line 99999");
    }

    #[test]
    fn latest_picks_the_newest_matching_file() {
        let dir = tempfile::tempdir().unwrap();
        let folder = fs::canonicalize(dir.path()).unwrap();
        for (name, age) in [("old_last.log", 30), ("new_last.log", 0), ("other.txt", 0)] {
            let file = File::create(folder.join(name)).unwrap();
            let when = std::time::SystemTime::now() - std::time::Duration::from_secs(age);
            file.set_modified(when).unwrap();
        }
        let found = latest(&folder.to_string_lossy(), "_last.log").unwrap();
        assert!(found.as_str().unwrap().ends_with("new_last.log"), "{found}");
        assert_eq!(
            latest(&folder.to_string_lossy(), ".none").unwrap(),
            Value::Null
        );
    }

    #[test]
    fn invalid_utf8_does_not_fail() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(&dir, b"ok\xff\xfe\nnext\n");
        assert_eq!(tail(&path, 2).unwrap().as_array().unwrap().len(), 2);
    }

    #[test]
    fn missing_files_and_alternate_spellings_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(&dir, b"a\n");
        assert!(tail(&format!("{path}.missing"), 1).is_err());
        let dotted = path.replace("game.log", "./game.log");
        assert!(tail(&dotted, 1).is_err());
    }
}
