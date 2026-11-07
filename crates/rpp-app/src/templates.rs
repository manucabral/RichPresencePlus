//! New presence folders: written from a template, or copied from another presence.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use rpp_plugins::{MAX_NAME, Manifest, reserved_device, valid_name};
use serde::Deserialize;

/// Creations run one at a time, so a stale staging folder is never removed under another request.
static BUSY: Mutex<()> = Mutex::new(());

const EXISTS: &str = "A presence folder with that name already exists.";
const BAD_NAME: &str = "Use letters, digits, spaces, _ - and . in the name (up to 64).";

/// Which kind of presence a template starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Template {
    /// Reads a web page through the browser extension.
    Web,
    /// Looks at the programs running on the computer.
    Desktop,
}

struct Files {
    manifest: &'static str,
    script: &'static str,
    english: &'static str,
    spanish: &'static str,
}

impl Template {
    fn files(self) -> Files {
        match self {
            Template::Web => Files {
                manifest: include_str!("../templates/web/manifest.json"),
                script: include_str!("../templates/web/main.js"),
                english: include_str!("../templates/web/locales/en.json"),
                spanish: include_str!("../templates/web/locales/es.json"),
            },
            Template::Desktop => Files {
                manifest: include_str!("../templates/desktop/manifest.json"),
                script: include_str!("../templates/desktop/main.js"),
                english: include_str!("../templates/desktop/locales/en.json"),
                spanish: include_str!("../templates/desktop/locales/es.json"),
            },
        }
    }
}

/// The folder name for a presence name: lower-case letters, digits and
/// hyphens. `None` when nothing usable is left of the name.
pub fn folder_name(name: &str) -> Option<String> {
    let mut folder = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            folder.push(c);
        } else if matches!(c, ' ' | '_' | '-' | '.') && !folder.ends_with('-') {
            folder.push('-');
        }
    }
    let folder = folder.trim_matches('-');
    (!folder.is_empty() && folder.len() <= MAX_NAME).then(|| folder.to_owned())
}

/// The trimmed name and its folder, or why the name cannot be used.
pub(crate) fn checked(name: &str) -> Result<(&str, String), String> {
    let name = name.trim();
    if !valid_name(name) {
        return Err(BAD_NAME.into());
    }
    let folder = folder_name(name).ok_or(
        "The name needs at least one letter from A to Z or a digit, to make the folder name.",
    )?;
    if reserved_device(&folder) {
        return Err("That name is reserved by Windows. Pick another.".into());
    }
    Ok((name, folder))
}

/// Builds a folder in a hidden place and renames it to `folder`.
pub(crate) fn publish(
    root: &Path,
    folder: &str,
    fill: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<PathBuf, String> {
    let _one_at_a_time = BUSY.lock().unwrap_or_else(PoisonError::into_inner);
    let target = root.join(folder);
    if target.exists() {
        return Err(EXISTS.into());
    }
    let staging = root.join(format!(".creating-{folder}"));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&staging).map_err(|e| format!("Cannot write the presence: {e}"))?;
    let built = fill(&staging).and_then(|()| {
        fs::rename(&staging, &target).map_err(|e| {
            if target.exists() {
                EXISTS.to_owned()
            } else {
                format!("Cannot write the presence: {e}")
            }
        })
    });
    if let Err(error) = built {
        // A half-written folder would load as a broken presence.
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    Ok(target)
}

/// Writes a new presence folder from a template and returns it.
///
/// Raises:
///     String: The name cannot be used or the folder already exists.
pub fn create(
    root: &Path,
    name: &str,
    template: Template,
    author: Option<&str>,
) -> Result<PathBuf, String> {
    let (name, folder) = checked(name)?;
    let files = template.files();
    let mut manifest = files.manifest.replace("__NAME__", name);
    if let Some(author) = author {
        let mut document: serde_json::Value =
            serde_json::from_str(&manifest).map_err(|e| e.to_string())?;
        if let Some(object) = document.as_object_mut() {
            object.insert(
                "author".into(),
                serde_json::Value::String(author.to_owned()),
            );
        }
        manifest = serde_json::to_string_pretty(&document).map_err(|e| e.to_string())? + "\n";
    }
    // The same check the application applies when it loads the folder.
    Manifest::parse(&manifest).map_err(|e| e.to_string())?;
    let script = files.script.replace("__NAME__", name);
    publish(root, &folder, |staging| {
        write(
            staging,
            &[
                ("manifest.json", &manifest),
                ("main.js", &script),
                ("locales/en.json", files.english),
                ("locales/es.json", files.spanish),
            ],
        )
        .map_err(|e| format!("Cannot write the presence: {e}"))
    })
}

/// Most files and bytes a copied presence may have, as the catalog allows.
pub(crate) const COPY_FILES: usize = 128;
pub(crate) const COPY_BYTES: u64 = 8 * 1024 * 1024;
/// Most names in `contributors`, as the manifest allows.
const MAX_CONTRIBUTORS: usize = 10;

/// Copies a presence folder under a new name, so it can be edited.
///
/// Raises:
///     String: The name cannot be used, the folder exists or the copy is too large.
pub fn copy(
    root: &Path,
    source: &Path,
    name: &str,
    author: Option<&str>,
) -> Result<PathBuf, String> {
    let (name, folder) = checked(name)?;
    publish(root, &folder, |staging| {
        copy_tree(source, staging, &mut (0, 0))?;
        relabel(staging, name, author)
    })
}

fn copy_tree(from: &Path, to: &Path, spent: &mut (usize, u64)) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let name = entry.file_name();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let editor = name.to_string_lossy().starts_with('.') || name == crate::editor::JSCONFIG;
        if editor || kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            copy_tree(&entry.path(), &to.join(&name), spent)?;
        } else if kind.is_file() {
            let size = entry.metadata().map_err(|e| e.to_string())?.len();
            spent.0 += 1;
            spent.1 += size;
            if spent.0 > COPY_FILES || spent.1 > COPY_BYTES {
                return Err("The presence is too large to copy.".into());
            }
            fs::copy(entry.path(), to.join(&name)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Gives the copied manifest its new name and moves the credit, keeping every other field.
fn relabel(folder: &Path, name: &str, author: Option<&str>) -> Result<(), String> {
    let path = folder.join("manifest.json");
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut document: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let object = document
        .as_object_mut()
        .ok_or("The manifest is not a JSON object.")?;
    object.insert("name".into(), serde_json::Value::String(name.to_owned()));

    let mut credit: Vec<String> = Vec::new();
    for field in ["author", "contributors"] {
        match object.remove(field) {
            Some(serde_json::Value::String(person)) => credit.push(person),
            Some(serde_json::Value::Array(people)) => {
                credit.extend(
                    people
                        .into_iter()
                        .filter_map(|person| person.as_str().map(str::to_owned)),
                );
            }
            _ => {}
        }
    }
    let mut seen = std::collections::HashSet::new();
    if let Some(author) = author {
        seen.insert(author.to_lowercase());
        object.insert(
            "author".into(),
            serde_json::Value::String(author.to_owned()),
        );
    }
    credit.retain(|person| seen.insert(person.to_lowercase()));
    credit.truncate(MAX_CONTRIBUTORS);
    if !credit.is_empty() {
        object.insert("contributors".into(), serde_json::json!(credit));
    }

    let renamed = serde_json::to_string_pretty(&document).map_err(|e| e.to_string())? + "\n";
    Manifest::parse(&renamed).map_err(|e| e.to_string())?;
    fs::write(path, renamed).map_err(|e| e.to_string())
}

pub(crate) fn write(folder: &Path, files: &[(&str, &str)]) -> std::io::Result<()> {
    for (path, text) in files {
        let target = folder.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use rpp_config::load_presences;
    use rpp_plugins::Plugin;
    use rpp_testkit::Processes;
    use serde_json::json;

    use super::*;

    fn load(root: &Path) -> (Plugin, std::collections::BTreeMap<String, String>) {
        let report = load_presences(&[root.to_owned()]);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let presence = report.presences.into_iter().next().unwrap();
        let strings = report.locales[&presence.manifest.name]["en"].clone();
        let host = Processes(vec!["Example.exe".into()]);
        let mut plugin = Plugin::load(presence.manifest, &presence.source, Rc::new(host)).unwrap();
        plugin.set_strings(&strings).unwrap();
        (plugin, strings)
    }

    #[test]
    fn folder_names_are_plain() {
        assert_eq!(folder_name("My Site").as_deref(), Some("my-site"));
        assert_eq!(folder_name("  A__b..C ").as_deref(), Some("a-b-c"));
        assert_eq!(folder_name("Café 2").as_deref(), Some("caf-2"));
        assert_eq!(folder_name("---"), None);
        assert_eq!(folder_name("日本語"), None);
    }

    #[test]
    fn the_web_template_loads_and_follows_its_option() {
        let root = tempfile::tempdir().unwrap();
        let folder = create(root.path(), "My Site", Template::Web, None).unwrap();
        assert_eq!(folder, root.path().join("my-site"));
        let (mut plugin, _) = load(root.path());
        let page = json!([{
            "url": "https://www.example.com/watch?v=1", "title": "Tab title",
            "received_at": 1, "values": {"title": "Heading"}
        }]);
        let shown = plugin.tick(&json!({"pages": page})).unwrap().unwrap();
        assert_eq!(shown.details.as_deref(), Some("Heading"));
        assert_eq!(shown.state.as_deref(), Some("www.example.com"));

        let hidden = plugin
            .tick(&json!({"pages": page, "options": {"include_page_title": false}}))
            .unwrap()
            .unwrap();
        assert_eq!(hidden.details.as_deref(), Some("www.example.com"));
        assert_eq!(plugin.tick(&json!({"pages": []})).unwrap(), None);
    }

    #[test]
    fn the_desktop_template_loads_and_follows_the_program() {
        let root = tempfile::tempdir().unwrap();
        create(root.path(), "My Game", Template::Desktop, None).unwrap();
        let (mut plugin, _) = load(root.path());
        let shown = plugin.tick(&json!({})).unwrap().unwrap();
        assert_eq!(shown.name.as_deref(), Some("My Game"));
        assert_eq!(shown.state.as_deref(), Some("Editing..."));
        let quiet = plugin
            .tick(&json!({"options": {"show_state": false}}))
            .unwrap()
            .unwrap();
        assert_eq!(quiet.state, None);

        // Without the program there is nothing to show.
        let report = load_presences(&[root.path().to_owned()]);
        let presence = report.presences.into_iter().next().unwrap();
        let mut closed = Plugin::load(
            presence.manifest,
            &presence.source,
            Rc::new(Processes(vec!["explorer.exe".into()])),
        )
        .unwrap();
        assert_eq!(closed.tick(&json!({})).unwrap(), None);
    }

    #[test]
    fn existing_folders_and_unusable_names_are_refused() {
        let root = tempfile::tempdir().unwrap();
        create(root.path(), "Site", Template::Web, None).unwrap();
        assert!(create(root.path(), "site", Template::Web, None).is_err());
        // The same presence spelled with other separators lands on the same folder.
        create(root.path(), "My Site", Template::Web, None).unwrap();
        for variant in ["my_site", "MY.SITE", "my  -  site"] {
            assert!(
                create(root.path(), variant, Template::Web, None).is_err(),
                "{variant}"
            );
        }
        for bad in ["", "   ", "../x", "a/b", "with \"quote\"", ".hidden", "---"] {
            assert!(
                create(root.path(), bad, Template::Web, None).is_err(),
                "{bad:?}"
            );
        }
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[test]
    fn windows_device_names_are_refused() {
        let root = tempfile::tempdir().unwrap();
        for reserved in ["con", "NUL", "Com1", "lpt9", "aux", "prn"] {
            let error = create(root.path(), reserved, Template::Web, None).unwrap_err();
            assert!(error.contains("reserved"), "{reserved}: {error}");
        }
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        // Only the whole name is reserved.
        assert!(create(root.path(), "Console", Template::Web, None).is_ok());
    }

    #[test]
    fn a_stale_staging_folder_does_not_block_a_creation_and_nothing_is_left() {
        let root = tempfile::tempdir().unwrap();
        let stale = root.path().join(".creating-site");
        fs::create_dir_all(stale.join("junk")).unwrap();
        create(root.path(), "Site", Template::Web, None).unwrap();
        assert!(!stale.exists());
        assert!(root.path().join("site/manifest.json").is_file());
    }

    #[test]
    fn two_requests_for_one_name_cannot_both_succeed() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().to_owned();
        let results: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || create(&path, "Race", Template::Web, None))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert!(
            results
                .iter()
                .filter_map(|r| r.as_ref().err())
                .all(|e| e == EXISTS)
        );
        assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
    }

    #[test]
    fn the_templates_ship_every_language_the_application_does() {
        for template in [Template::Web, Template::Desktop] {
            let files = template.files();
            let english: std::collections::BTreeMap<String, String> =
                serde_json::from_str(files.english).unwrap();
            let spanish: std::collections::BTreeMap<String, String> =
                serde_json::from_str(files.spanish).unwrap();
            assert_eq!(
                english.keys().collect::<Vec<_>>(),
                spanish.keys().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn a_copy_gets_a_new_name_and_leaves_out_hidden_files() {
        let root = tempfile::tempdir().unwrap();
        let original = create(root.path(), "Original", Template::Web, None).unwrap();
        fs::write(original.join(".catalog.json"), "{}").unwrap();
        fs::write(original.join("notes.txt"), "kept").unwrap();

        let copy = copy(root.path(), &original, "My Copy", None).unwrap();
        assert_eq!(copy, root.path().join("my-copy"));
        assert!(copy.join("main.js").is_file());
        assert!(copy.join("locales").join("es.json").is_file());
        assert_eq!(fs::read_to_string(copy.join("notes.txt")).unwrap(), "kept");
        assert!(!copy.join(".catalog.json").exists());

        let report = load_presences(&[root.path().to_owned()]);
        assert!(report.problems.is_empty());
        let names: Vec<_> = report
            .presences
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect();
        assert_eq!(names, ["My Copy", "Original"]);
    }

    #[test]
    fn a_copy_does_not_carry_the_author_of_the_original() {
        let root = tempfile::tempdir().unwrap();
        let original = create(root.path(), "Original", Template::Web, None).unwrap();
        let path = original.join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        manifest["author"] = json!("Ana");
        manifest["contributors"] = json!(["Bo", "ana", "Cy"]);
        fs::write(&path, manifest.to_string()).unwrap();

        let copy = copy(root.path(), &original, "Fork", None).unwrap();
        let forked: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(copy.join("manifest.json")).unwrap()).unwrap();
        assert!(forked.get("author").is_none());
        assert_eq!(forked["contributors"], json!(["Ana", "Bo", "Cy"]));
        assert_eq!(forked["name"], "Fork");
        // The original is untouched.
        assert!(fs::read_to_string(&path).unwrap().contains("\"author\""));
    }

    #[test]
    fn credit_in_a_copy_stays_within_the_manifest_limit() {
        let root = tempfile::tempdir().unwrap();
        let original = create(root.path(), "Original", Template::Web, None).unwrap();
        let path = original.join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let people: Vec<String> = (0..10).map(|n| format!("person {n}")).collect();
        manifest["author"] = json!("lead");
        manifest["contributors"] = json!(people);
        fs::write(&path, manifest.to_string()).unwrap();

        let copy = copy(root.path(), &original, "Fork", None).unwrap();
        let forked: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(copy.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(forked["contributors"].as_array().unwrap().len(), 10);
        assert_eq!(forked["contributors"][0], "lead");
    }

    #[test]
    fn a_copy_that_cannot_be_made_leaves_nothing_behind() {
        let root = tempfile::tempdir().unwrap();
        let original = create(root.path(), "Original", Template::Web, None).unwrap();
        assert!(copy(root.path(), &original, "original", None).is_err());
        assert!(copy(root.path(), &original, "../x", None).is_err());
        assert!(copy(root.path(), &original, "con", None).is_err());

        let big = tempfile::tempdir().unwrap();
        fs::write(big.path().join("manifest.json"), "{}").unwrap();
        for number in 0..=COPY_FILES {
            fs::write(big.path().join(format!("f{number}.js")), "x").unwrap();
        }
        assert!(copy(root.path(), big.path(), "Huge", None).is_err());
        assert!(!root.path().join("huge").exists());
        // A folder without a valid manifest is not copied either.
        let broken = tempfile::tempdir().unwrap();
        fs::write(broken.path().join("manifest.json"), "not json").unwrap();
        assert!(copy(root.path(), broken.path(), "Broken", None).is_err());
        assert!(!root.path().join("broken").exists());
        let leftovers: Vec<_> = fs::read_dir(root.path())
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, ["original"], "no staging folder remains");
    }
}

#[cfg(test)]
mod author_tests {
    use super::*;

    fn manifest(folder: &Path) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(folder.join("manifest.json")).unwrap()).unwrap()
    }

    #[test]
    fn both_templates_carry_the_author_when_one_is_given() {
        let root = tempfile::tempdir().unwrap();
        for (name, template) in [("Site", Template::Web), ("Game", Template::Desktop)] {
            let folder = create(root.path(), name, template, Some("manucabral")).unwrap();
            assert_eq!(manifest(&folder)["author"], "manucabral");
            assert!(
                Manifest::parse(&fs::read_to_string(folder.join("manifest.json")).unwrap()).is_ok()
            );
        }
        let plain = create(root.path(), "Plain", Template::Web, None).unwrap();
        assert!(manifest(&plain).get("author").is_none());
    }

    #[test]
    fn copying_your_own_presence_keeps_you_as_its_author() {
        let root = tempfile::tempdir().unwrap();
        let original = create(root.path(), "Original", Template::Web, Some("ManuCabral")).unwrap();
        let copy = copy(root.path(), &original, "Mine Again", Some("manucabral")).unwrap();
        let forked = manifest(&copy);
        assert_eq!(forked["author"], "manucabral");
        assert!(forked.get("contributors").is_none(), "{forked}");
    }

    #[test]
    fn copying_someone_elses_presence_credits_them_and_names_you() {
        let root = tempfile::tempdir().unwrap();
        let original = create(root.path(), "Theirs", Template::Web, Some("Ana")).unwrap();
        let copy = copy(root.path(), &original, "Fork", Some("manucabral")).unwrap();
        let forked = manifest(&copy);
        assert_eq!(forked["author"], "manucabral");
        assert_eq!(forked["contributors"], serde_json::json!(["Ana"]));
    }
}
