//! Editing the descriptive fields of a presence's `manifest.json`.

use std::fs;
use std::io::Write;
use std::path::Path;

use rpp_plugins::{Manifest, is_known_tag};
use serde::Deserialize;
use serde_json::{Map, Value};

/// What to change. A missing field is left alone; an empty text removes the field.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ManifestChanges {
    /// New version.
    pub version: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// New author.
    pub author: Option<String>,
    /// New color, as `#RRGGBB`.
    pub color: Option<String>,
    /// New tags; an empty list removes them.
    pub tags: Option<Vec<String>>,
}

fn set(object: &mut Map<String, Value>, key: &str, text: Option<&str>) {
    match text.map(str::trim) {
        None => {}
        Some("") => {
            object.remove(key);
        }
        Some(text) => {
            object.insert(key.to_owned(), Value::String(text.to_owned()));
        }
    }
}

/// Applies the changes to the manifest in a folder.
pub fn apply(folder: &Path, changes: &ManifestChanges) -> Result<(), String> {
    let path = folder.join("manifest.json");
    // A link could point at a file outside the folder, and the rename would replace the link.
    let kind = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !kind.file_type().is_file() {
        return Err("The manifest is not a plain file.".into());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut document: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let object = document
        .as_object_mut()
        .ok_or("The manifest is not a JSON object.")?;
    if matches!(changes.version.as_deref().map(str::trim), Some("")) {
        return Err("A presence needs a version.".into());
    }
    set(object, "version", changes.version.as_deref());
    set(object, "description", changes.description.as_deref());
    set(object, "author", changes.author.as_deref());
    set(object, "color", changes.color.as_deref());
    match &changes.tags {
        None => {}
        Some(tags) if tags.is_empty() => {
            object.remove("tags");
        }
        Some(tags) => {
            if let Some(unknown) = tags.iter().find(|tag| !is_known_tag(tag.trim())) {
                return Err(format!("{unknown} is not on the list of tags."));
            }
            object.insert(
                "tags".to_owned(),
                Value::Array(
                    tags.iter()
                        .map(|tag| Value::String(tag.trim().to_owned()))
                        .collect(),
                ),
            );
        }
    }
    let updated = serde_json::to_string_pretty(&document).map_err(|e| e.to_string())? + "\n";
    Manifest::parse(&updated).map_err(|e| e.to_string())?;

    let temporary = folder.join(".manifest.json.tmp");
    // Removing a leftover removes a planted link itself, never what it points at;
    // `create_new` then refuses to write through anything that appears meanwhile.
    let _ = fs::remove_file(&temporary);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let written = file
        .write_all(updated.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    let finished = written.and_then(|()| fs::rename(&temporary, &path));
    finished.map_err(|e| {
        let _ = fs::remove_file(&temporary);
        e.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL: &str = r#"{
  "name": "Demo",
  "version": "1.0.0",
  "client_id": "12345678901234567",
  "interval": 5,
  "permissions": { "process": true }
}"#;

    fn folder() -> tempfile::TempDir {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("manifest.json"), ORIGINAL).unwrap();
        folder
    }

    fn read(folder: &Path) -> Value {
        serde_json::from_str(&fs::read_to_string(folder.join("manifest.json")).unwrap()).unwrap()
    }

    #[test]
    fn only_the_named_fields_change_and_the_rest_keeps_its_place() {
        let folder = folder();
        apply(
            folder.path(),
            &ManifestChanges {
                version: Some("1.1.0".into()),
                author: Some(" Ana ".into()),
                color: Some("#A56786".into()),
                tags: Some(vec!["video".into(), "music".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        let after = read(folder.path());
        assert_eq!(after["version"], "1.1.0");
        assert_eq!(after["author"], "Ana");
        assert_eq!(after["tags"], serde_json::json!(["video", "music"]));
        assert_eq!(after["interval"], 5);
        assert_eq!(after["permissions"], serde_json::json!({"process": true}));
        let text = fs::read_to_string(folder.path().join("manifest.json")).unwrap();
        let order: Vec<_> = ["\"name\"", "\"version\"", "\"client_id\"", "\"interval\""]
            .iter()
            .map(|key| text.find(key).unwrap())
            .collect();
        assert!(
            order.windows(2).all(|pair| pair[0] < pair[1]),
            "keys keep their order"
        );
        assert!(!folder.path().join(".manifest.json.tmp").exists());
    }

    #[test]
    fn an_empty_text_removes_a_field() {
        let folder = folder();
        let set_all = ManifestChanges {
            description: Some("Shows things.".into()),
            author: Some("Ana".into()),
            tags: Some(vec!["video".into()]),
            ..Default::default()
        };
        apply(folder.path(), &set_all).unwrap();
        let removed = ManifestChanges {
            description: Some(String::new()),
            author: Some("  ".into()),
            tags: Some(Vec::new()),
            ..Default::default()
        };
        apply(folder.path(), &removed).unwrap();
        let after = read(folder.path());
        for gone in ["description", "author", "tags"] {
            assert!(after.get(gone).is_none(), "{gone}");
        }
    }

    #[test]
    fn a_leftover_temporary_file_is_replaced_not_followed() {
        let folder = folder();
        let temporary = folder.path().join(".manifest.json.tmp");
        fs::write(&temporary, "left by a crash").unwrap();
        let change = ManifestChanges {
            author: Some("Ana".into()),
            ..Default::default()
        };
        apply(folder.path(), &change).unwrap();
        assert_eq!(read(folder.path())["author"], "Ana");
        assert!(!temporary.exists());

        // A link planted at the temporary path must not receive the manifest.
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("victim.txt");
        fs::write(&target, "untouched").unwrap();
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_file(&target, &temporary);
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&target, &temporary);
        // Creating a link needs a privilege on some systems; without it there is nothing to test.
        if linked.is_ok() {
            apply(folder.path(), &change).unwrap();
            assert_eq!(fs::read_to_string(&target).unwrap(), "untouched");
            assert!(!temporary.exists());
        }
    }

    #[test]
    fn a_manifest_that_is_not_a_plain_file_is_refused() {
        let folder = tempfile::tempdir().unwrap();
        fs::create_dir(folder.path().join("manifest.json")).unwrap();
        assert!(apply(folder.path(), &ManifestChanges::default()).is_err());

        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("manifest.json");
        fs::write(&target, ORIGINAL).unwrap();
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("manifest.json");
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_file(&target, &path);
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&target, &path);
        if linked.is_ok() {
            let change = ManifestChanges {
                author: Some("Ana".into()),
                ..Default::default()
            };
            assert!(apply(folder.path(), &change).is_err());
            assert_eq!(fs::read_to_string(&target).unwrap(), ORIGINAL);
        }
    }

    #[test]
    fn a_refused_edit_leaves_the_file_exactly_as_it_was() {
        let folder = folder();
        let bad = [
            ManifestChanges {
                color: Some("red".into()),
                ..Default::default()
            },
            ManifestChanges {
                tags: Some(vec!["Bad Tag".into()]),
                ..Default::default()
            },
            ManifestChanges {
                tags: Some(vec!["invented".into()]),
                ..Default::default()
            },
            ManifestChanges {
                version: Some("  ".into()),
                ..Default::default()
            },
        ];
        for changes in bad {
            assert!(apply(folder.path(), &changes).is_err());
            let text = fs::read_to_string(folder.path().join("manifest.json")).unwrap();
            assert_eq!(text, ORIGINAL);
        }
        assert!(apply(&folder.path().join("missing"), &ManifestChanges::default()).is_err());
    }
}
