//! One file that carries every file of a presence version.

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;

use crate::error::Error;
use crate::index::Entry;
use crate::install::sha256_hex;

/// Value of the `format` field.
pub const FORMAT: &str = "rich-presence-plus/bundle";
/// Hexadecimal digits of the content hash in a bundle's name.
const HASH_DIGITS: usize = 12;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: String,
    folder: String,
    version: String,
    files: BTreeMap<String, String>,
}

/// Whether `name` is the release file name the catalog tool gives a bundle of
/// `folder` at `version`: `<folder>_<version>_<12 hex digits>.json`.
pub fn valid_name(name: &str, folder: &str, version: &str) -> bool {
    if version.contains('+') {
        return false;
    }
    name.strip_prefix(folder)
        .and_then(|rest| rest.strip_prefix('_'))
        .and_then(|rest| rest.strip_prefix(version))
        .and_then(|rest| rest.strip_prefix('_'))
        .and_then(|rest| rest.strip_suffix(".json"))
        .is_some_and(|hash| {
            hash.len() == HASH_DIGITS
                && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
}

/// Largest bundle accepted for an entry: its files in base64 plus room for
/// the JSON around them. A longer download is cut there and then refused.
pub fn limit(entry: &Entry) -> u64 {
    let encoded: u64 = entry
        .files
        .iter()
        .map(|(path, info)| info.size.div_ceil(3) * 4 + path.len() as u64 + 8)
        .sum();
    encoded + 1024
}

/// Opens a bundle, checks it against its entry in the verified index and
/// returns every file by path.
///
/// Raises:
///     Error: `Invalid` for a wrong shape, presence or version, or a different
///         set of files; `Integrity` for a file whose size or digest differs.
pub fn unpack(bytes: &[u8], entry: &Entry) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    if bytes.len() as u64 > limit(entry) {
        return Err(Error::Invalid(format!(
            "{}: the bundle is too large",
            entry.folder
        )));
    }
    let raw: Raw = serde_json::from_slice(bytes)
        .map_err(|_| Error::Invalid(format!("{}: the bundle is not readable", entry.folder)))?;
    if raw.format != FORMAT
        || raw.folder != entry.folder
        || raw.version != entry.version.to_string()
    {
        return Err(Error::Invalid(format!(
            "{}: the bundle is for another presence or version",
            entry.folder
        )));
    }
    if !raw.files.keys().eq(entry.files.keys()) {
        return Err(Error::Invalid(format!(
            "{}: the bundle does not hold the signed files",
            entry.folder
        )));
    }
    let mut out = BTreeMap::new();
    for (path, encoded) in raw.files {
        let info = &entry.files[&path];
        let content = STANDARD
            .decode(encoded)
            .map_err(|_| Error::Integrity(format!("{}/{path}", entry.folder)))?;
        if content.len() as u64 != info.size || sha256_hex(&content) != info.sha256 {
            return Err(Error::Integrity(format!("{}/{path}", entry.folder)));
        }
        out.insert(path, content);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::FileInfo;
    use semver::Version;
    use serde_json::json;

    fn entry(files: &[(&str, &[u8])]) -> Entry {
        Entry {
            name: "Demo".into(),
            folder: "demo".into(),
            version: Version::new(1, 2, 0),
            description: None,
            kind: None,
            files: files
                .iter()
                .map(|(path, bytes)| {
                    let info = FileInfo {
                        sha256: sha256_hex(bytes),
                        size: bytes.len() as u64,
                    };
                    ((*path).to_owned(), info)
                })
                .collect(),
            author: None,
            color: None,
            tags: Vec::new(),
            icon: None,
            bundle: None,
        }
    }

    fn bundle(folder: &str, version: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
        let files: BTreeMap<_, _> = files
            .iter()
            .map(|(path, bytes)| ((*path).to_owned(), STANDARD.encode(bytes)))
            .collect();
        json!({ "format": FORMAT, "folder": folder, "version": version, "files": files })
            .to_string()
            .into_bytes()
    }

    const FILES: [(&str, &[u8]); 2] = [("manifest.json", b"{}"), ("main.js", b"function read(){}")];

    #[test]
    fn a_bundle_that_matches_the_index_is_opened() {
        let opened = unpack(&bundle("demo", "1.2.0", &FILES), &entry(&FILES)).unwrap();
        assert_eq!(opened.len(), 2);
        assert_eq!(opened["main.js"], b"function read(){}");
    }

    #[test]
    fn any_difference_from_the_index_is_refused() {
        let signed = entry(&FILES);
        // Another presence, another version, another format.
        assert!(unpack(&bundle("other", "1.2.0", &FILES), &signed).is_err());
        assert!(unpack(&bundle("demo", "1.1.0", &FILES), &signed).is_err());
        let mut wrong =
            serde_json::from_slice::<serde_json::Value>(&bundle("demo", "1.2.0", &FILES)).unwrap();
        wrong["format"] = json!("zip");
        assert!(unpack(wrong.to_string().as_bytes(), &signed).is_err());
        // A file missing, a file added, a file changed.
        assert!(unpack(&bundle("demo", "1.2.0", &FILES[..1]), &signed).is_err());
        let extra = [FILES[0], FILES[1], ("evil.js", b"x".as_slice())];
        assert!(unpack(&bundle("demo", "1.2.0", &extra), &signed).is_err());
        let changed = [
            FILES[0],
            ("main.js", b"function read(){ steal() }".as_slice()),
        ];
        assert!(matches!(
            unpack(&bundle("demo", "1.2.0", &changed), &signed),
            Err(Error::Integrity(_))
        ));
        // Unknown fields, bad base64 and plain garbage.
        let mut unknown =
            serde_json::from_slice::<serde_json::Value>(&bundle("demo", "1.2.0", &FILES)).unwrap();
        unknown["run"] = json!("main.js");
        assert!(unpack(unknown.to_string().as_bytes(), &signed).is_err());
        let mut broken =
            serde_json::from_slice::<serde_json::Value>(&bundle("demo", "1.2.0", &FILES)).unwrap();
        broken["files"]["main.js"] = json!("***");
        assert!(unpack(broken.to_string().as_bytes(), &signed).is_err());
        assert!(unpack(b"not json", &signed).is_err());
    }

    #[test]
    fn an_oversized_bundle_is_refused_before_parsing() {
        let signed = entry(&FILES);
        let mut big = bundle("demo", "1.2.0", &FILES);
        big.extend(std::iter::repeat_n(b' ', limit(&signed) as usize));
        assert!(unpack(&big, &signed).is_err());
    }

    #[test]
    fn names_split_back_into_folder_version_and_hash() {
        assert!(valid_name("demo_1.2.0_0123456789ab.json", "demo", "1.2.0"));
        assert!(valid_name(
            "my-site_2.0.0-rc.1_abcdefabcdef.json",
            "my-site",
            "2.0.0-rc.1"
        ));
        for bad in [
            "demo_1.2.0_0123456789ab.zip",
            "demo_1.2.0_0123456789a.json",
            "demo_1.2.0_0123456789AB.json",
            "demo_1.2.1_0123456789ab.json",
            "other_1.2.0_0123456789ab.json",
            "../demo_1.2.0_0123456789ab.json",
            "demo_1.2.0_0123456789ab.json/x",
        ] {
            assert!(!valid_name(bad, "demo", "1.2.0"), "{bad}");
        }
        assert!(!valid_name(
            "demo_1.2.0+b1_0123456789ab.json",
            "demo",
            "1.2.0+b1"
        ));
    }
}
