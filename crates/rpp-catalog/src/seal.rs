//! A seal on the marker of an installed presence.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::install::{Installed, sha256_hex};

/// Name of the file that holds the secret, inside the application's data folder.
const KEY_FILE: &str = "catalog.key";
/// Shortest secret accepted.
const MIN_KEY: usize = 32;
/// Files, depth and size scanned in a presence folder, as the index allows.
const MAX_FILES: usize = 128;
const MAX_DEPTH: usize = 4;
const MAX_FILE: u64 = 1024 * 1024;
/// Files the operating system adds on its own; they are not part of a presence.
const OS_FILES: [&str; 2] = ["thumbs.db", "desktop.ini"];

/// The secret that seals the markers of this installation.
#[derive(Clone)]
pub struct Key(Vec<u8>);

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key(..)")
    }
}

impl Key {
    /// Uses a text as the secret; `None` when it is shorter than 32 characters.
    pub fn new(secret: &str) -> Option<Self> {
        (secret.len() >= MIN_KEY).then(|| Self(secret.as_bytes().to_vec()))
    }

    /// Reads the secret kept in a folder, creating it the first time.
    pub fn load_or_create(dir: &Path) -> Self {
        let path = dir.join(KEY_FILE);
        if let Some(key) = fs::read_to_string(&path)
            .ok()
            .and_then(|text| Self::new(text.trim()))
        {
            return key;
        }
        let mut bytes = [0_u8; 32];
        if getrandom::fill(&mut bytes).is_err() {
            // No random numbers: seal for this run only and keep nothing on disk.
            let now = std::time::SystemTime::now();
            return Self(format!("{now:?}-{}", std::process::id()).into_bytes());
        }
        let secret: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let _ = fs::create_dir_all(dir);
        let _ = fs::write(&path, &secret);
        Self(secret.into_bytes())
    }
}

/// The text the seal covers: the record, then every file in path order.
fn message(record: &Installed) -> Vec<u8> {
    let mut text = format!(
        "{}\0{}\0{}\0{}\0",
        record.name, record.folder, record.version, record.commit
    );
    for (path, digest) in &record.files {
        text.push_str(&format!("{path}\0{digest}\0"));
    }
    text.into_bytes()
}

fn mac(key: &Key, record: &Installed) -> Option<Hmac<Sha256>> {
    // HMAC takes a key of any length, so this does not fail in practice.
    let mut mac = <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(&key.0).ok()?;
    mac.update(&message(record));
    Some(mac)
}

/// The seal for a record, as lower-case hexadecimal; empty if it cannot be made.
pub(crate) fn sign(key: &Key, record: &Installed) -> String {
    mac(key, record).map_or_else(String::new, |mac| {
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    })
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).ok())
        .collect()
}

/// Tells whether the seal of a record is the one this key makes for it.
pub(crate) fn verifies(key: &Key, record: &Installed) -> bool {
    let Some(mac) = mac(key, record) else {
        return false;
    };
    unhex(&record.seal).is_some_and(|seal| mac.verify_slice(&seal).is_ok())
}

/// The digest of every visible file in a folder, by path.
///
/// Returns:
///     `None` when the folder holds a link, too many files, a file that is too
///     large or nesting that is too deep: none of that comes from the catalog.
pub(crate) fn scan(folder: &Path) -> Option<BTreeMap<String, String>> {
    let mut found = BTreeMap::new();
    walk(folder, &PathBuf::new(), &mut found)?;
    Some(found)
}

fn walk(base: &Path, relative: &Path, found: &mut BTreeMap<String, String>) -> Option<()> {
    if relative.components().count() > MAX_DEPTH {
        return None;
    }
    for entry in fs::read_dir(base.join(relative)).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || OS_FILES.contains(&name.to_lowercase().as_str()) {
            continue;
        }
        let kind = entry.file_type().ok()?;
        let path = relative.join(&name);
        if kind.is_dir() {
            walk(base, &path, found)?;
        } else if kind.is_file() {
            if found.len() >= MAX_FILES || entry.metadata().ok()?.len() > MAX_FILE {
                return None;
            }
            let digest = sha256_hex(&fs::read(entry.path()).ok()?);
            found.insert(path.to_string_lossy().replace('\\', "/"), digest);
        } else {
            return None;
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::Standing;

    fn record() -> Installed {
        Installed {
            name: "Demo".into(),
            folder: "demo".into(),
            version: "1.0.0".parse().unwrap(),
            commit: "a".repeat(40),
            files: BTreeMap::from([("main.js".to_owned(), "ab".repeat(32))]),
            seal: String::new(),
            standing: Standing::Intact,
        }
    }

    fn key(letter: &str) -> Key {
        Key::new(&letter.repeat(40)).unwrap()
    }

    #[test]
    fn a_seal_covers_every_field_and_belongs_to_one_key() {
        let mut sealed = record();
        sealed.seal = sign(&key("a"), &sealed);
        assert!(verifies(&key("a"), &sealed));
        assert!(!verifies(&key("b"), &sealed));
        let changes: [fn(&mut Installed); 5] = [
            |r| r.name = "Other".into(),
            |r| r.folder = "other".into(),
            |r| r.version = "1.0.1".parse().unwrap(),
            |r| r.commit = "b".repeat(40),
            |r| {
                r.files.insert("extra.js".into(), "cd".repeat(32));
            },
        ];
        for change in changes {
            let mut edited = sealed.clone();
            change(&mut edited);
            assert!(!verifies(&key("a"), &edited));
        }
        let mut digest = sealed.clone();
        digest.files.insert("main.js".into(), "cd".repeat(32));
        assert!(!verifies(&key("a"), &digest));
    }

    #[test]
    fn a_seal_that_is_missing_or_not_hexadecimal_never_verifies() {
        for seal in ["", "zz", "abc", "é", &"0".repeat(64)] {
            let mut forged = record();
            forged.seal = seal.to_owned();
            assert!(!verifies(&key("a"), &forged), "{seal:?}");
        }
    }

    #[test]
    fn short_secrets_are_refused() {
        assert!(Key::new(&"x".repeat(31)).is_none());
        assert!(Key::new(&"x".repeat(32)).is_some());
    }

    #[test]
    fn the_secret_is_created_once_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let first = Key::load_or_create(dir.path());
        let second = Key::load_or_create(dir.path());
        let mut sealed = record();
        sealed.seal = sign(&first, &sealed);
        assert!(verifies(&second, &sealed));
        assert_eq!(
            fs::read_to_string(dir.path().join(KEY_FILE)).unwrap().len(),
            64
        );
        // A damaged file is replaced instead of weakening the seal.
        fs::write(dir.path().join(KEY_FILE), "short").unwrap();
        let third = Key::load_or_create(dir.path());
        assert!(!verifies(&third, &sealed));
        assert_eq!(
            fs::read_to_string(dir.path().join(KEY_FILE)).unwrap().len(),
            64
        );
    }

    #[test]
    fn a_folder_is_read_by_its_visible_files_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("locales")).unwrap();
        fs::write(dir.path().join("main.js"), "a").unwrap();
        fs::write(dir.path().join("locales/en.json"), "{}").unwrap();
        fs::write(dir.path().join(".catalog.json"), "marker").unwrap();
        fs::write(dir.path().join("Thumbs.db"), "x").unwrap();
        let found = scan(dir.path()).unwrap();
        assert_eq!(
            found.keys().collect::<Vec<_>>(),
            ["locales/en.json", "main.js"]
        );
        assert_eq!(found["main.js"], sha256_hex(b"a"));
    }

    #[test]
    fn a_folder_beyond_the_index_limits_is_not_read() {
        let many = tempfile::tempdir().unwrap();
        for number in 0..=MAX_FILES {
            fs::write(many.path().join(format!("f{number}.js")), "x").unwrap();
        }
        assert!(scan(many.path()).is_none());

        let big = tempfile::tempdir().unwrap();
        fs::write(
            big.path().join("big.js"),
            vec![b'x'; (MAX_FILE + 1) as usize],
        )
        .unwrap();
        assert!(scan(big.path()).is_none());

        let deep = tempfile::tempdir().unwrap();
        let nested = deep.path().join("a/b/c/d/e");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("x.js"), "x").unwrap();
        assert!(scan(deep.path()).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn a_link_in_the_folder_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("x.js"), "x").unwrap();
        // Creating a link needs a privilege on some systems; without it there is nothing to test.
        if std::os::windows::fs::symlink_file(outside.path().join("x.js"), dir.path().join("x.js"))
            .is_ok()
        {
            assert!(scan(dir.path()).is_none());
        }
    }
}
