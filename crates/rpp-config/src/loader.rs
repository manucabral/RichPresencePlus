//! Finding presences in folders.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use rpp_core::locale::{self, Table};
use rpp_plugins::{Manifest, identity};
use rpp_service::Presence;
use serde::Serialize;

/// Largest manifest read.
const MAX_MANIFEST: u64 = 64 * 1024;
/// Largest script read.
const MAX_SCRIPT: u64 = 1024 * 1024;
/// Largest translation file read.
const MAX_LOCALE: u64 = 64 * 1024;
/// Most translation files read for one presence.
const MAX_LOCALES: usize = 100;

/// A folder that could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
    /// Folder name.
    pub folder: String,
    /// Why it was skipped.
    pub message: String,
}

/// The presences found and the folders that failed.
#[derive(Debug, Default)]
pub struct LoadReport {
    /// Presences ready to run, sorted by name.
    pub presences: Vec<Presence>,
    /// Folders that were skipped.
    pub problems: Vec<Problem>,
    /// Translations of each presence, by presence name and language tag.
    pub locales: HashMap<String, BTreeMap<String, Table>>,
    /// The folder each presence was read from, by presence name.
    pub folders: HashMap<String, PathBuf>,
    /// Where each presence came from, by presence name.
    pub origins: HashMap<String, Origin>,
}

fn read_limited(path: &Path, limit: u64) -> Result<String, String> {
    let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > limit {
        return Err(format!("{} is larger than {limit} bytes", path.display()));
    }
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Two folders hold the same presence when name, version, application and
/// script are identical, as with a bundled copy and the one in the repository.
fn same_presence(a: &Presence, b: &Presence) -> bool {
    a.manifest.name == b.manifest.name
        && a.manifest.version == b.manifest.version
        && a.manifest.client_id == b.manifest.client_id
        && a.source == b.source
}

/// Reads `locales/<tag>.json` files. A bad file is reported and skipped.
fn load_locales(folder: &Path, problems: &mut Vec<String>) -> BTreeMap<String, Table> {
    let mut tables = BTreeMap::new();
    let Ok(entries) = fs::read_dir(folder.join("locales")) else {
        return tables;
    };
    let mut files: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    files.sort();
    for path in files.into_iter().take(MAX_LOCALES) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let Some(tag) = locale::normalize(stem).filter(|tag| tag == stem) else {
            problems.push(format!(
                "locales/{stem}.json: not a language tag like en or pt-BR"
            ));
            continue;
        };
        match read_limited(&path, MAX_LOCALE)
            .and_then(|text| serde_json::from_str::<Table>(&text).map_err(|e| e.to_string()))
        {
            Ok(table) => {
                tables.insert(tag, table);
            }
            Err(message) => problems.push(format!("locales/{stem}.json: {message}")),
        }
    }
    tables
}

/// One presence folder, read.
#[derive(Debug)]
pub struct Found {
    /// The presence, without texts or options yet.
    pub presence: Presence,
    /// Its translations, by language tag.
    pub locales: BTreeMap<String, Table>,
    /// Translation files that were skipped, and why.
    pub warnings: Vec<String>,
}

/// Reads the presence folder that holds `manifest.json`.
///
/// Raises:
///     String: Why the folder cannot be used as a presence.
pub fn load_presence(folder: &Path) -> Result<Found, String> {
    load_one(folder, &|name| std::env::var(name).ok())
}

fn load_one(folder: &Path, lookup: &dyn Fn(&str) -> Option<String>) -> Result<Found, String> {
    let mut manifest = Manifest::parse(&read_limited(&folder.join("manifest.json"), MAX_MANIFEST)?)
        .map_err(|e| e.to_string())?;
    // Scripts see the real folders; a variable that does not exist drops its entry.
    manifest.permissions = manifest.permissions.resolve(lookup);
    let script = folder.join(&manifest.entry);
    // A link inside the folder must not lead to a file outside it.
    let inside = fs::canonicalize(&script)
        .and_then(|resolved| fs::canonicalize(folder).map(|root| resolved.starts_with(root)))
        .map_err(|e| format!("script {}: {e}", manifest.entry))?;
    if !inside {
        return Err("the script is outside the presence folder".into());
    }
    let source = read_limited(&script, MAX_SCRIPT)?;
    let mut warnings = Vec::new();
    let tables = load_locales(folder, &mut warnings);
    let presence = Presence {
        manifest,
        source,
        strings: Table::default(),
        settings: Default::default(),
    };
    Ok(Found {
        presence,
        locales: tables,
        warnings,
    })
}

/// Where a presence came from, which decides what it may do and who wins a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    /// Shipped with the application.
    Bundled,
    /// Installed from the catalog and unchanged since.
    Catalog,
    /// Written or copied by the user; the only kind that can be edited.
    Local,
}

/// Loads every presence found in the given folders.
///
/// Args:
///     roots: Folders with one subfolder per presence, in priority order.
///
/// Returns:
///     The presences and the folders that were skipped.
pub fn load_presences(roots: &[PathBuf]) -> LoadReport {
    load_presences_with(roots, &|name| std::env::var(name).ok())
}

/// Like [`load_presences`], reading environment variables through `lookup`.
pub fn load_presences_with(
    roots: &[PathBuf],
    lookup: &dyn Fn(&str) -> Option<String>,
) -> LoadReport {
    load_presences_ranked(roots, lookup, &|_| Ok(Origin::Bundled))
}

/// Like [`load_presences`], skipping the folders a check refuses.
///
/// Args:
///     refuse: Why a folder must not load, or `None` to load it.
pub fn load_presences_checked(
    roots: &[PathBuf],
    refuse: &dyn Fn(&Path) -> Option<String>,
) -> LoadReport {
    load_presences_ranked(roots, &|name| std::env::var(name).ok(), &|folder| {
        refuse(folder).map_or(Ok(Origin::Bundled), Err)
    })
}

/// A folder that was read and is waiting for its name to be settled.
struct Candidate {
    label: String,
    folder: PathBuf,
    origin: Origin,
    found: Found,
}

/// Like [`load_presences_with`], with each folder classified by where it came from.
///
/// Args:
///     classify: Says where a folder came from; an error skips the folder and
///         becomes its problem.
pub fn load_presences_ranked(
    roots: &[PathBuf],
    lookup: &dyn Fn(&str) -> Option<String>,
    classify: &dyn Fn(&Path) -> Result<Origin, String>,
) -> LoadReport {
    let mut report = LoadReport::default();
    let mut official = Vec::new();
    let mut local = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        let mut folders: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        folders.sort();
        for folder in folders {
            let label = folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if label.starts_with('.') {
                continue;
            }
            let read =
                classify(&folder).and_then(|origin| Ok((origin, load_one(&folder, lookup)?)));
            match read {
                Ok((origin, found)) => {
                    let candidate = Candidate {
                        label,
                        folder,
                        origin,
                        found,
                    };
                    if origin == Origin::Local {
                        local.push(candidate);
                    } else {
                        official.push(candidate);
                    }
                }
                Err(message) => report.problems.push(Problem {
                    folder: label,
                    message,
                }),
            }
        }
    }

    // Identity of each accepted name, with the name and folder that hold it.
    let mut taken: HashMap<String, (String, String)> = HashMap::new();
    for candidate in official.into_iter().chain(local) {
        let Candidate {
            label,
            folder,
            origin,
            found:
                Found {
                    presence,
                    locales,
                    warnings,
                },
        } = candidate;
        let name = presence.manifest.name.clone();
        if let Some((winner, winner_folder)) = taken.get(&identity(&name)) {
            // The same presence reachable by two paths is not a problem; a
            // local folder that merely looks like one is.
            let twin = origin != Origin::Local
                && report
                    .presences
                    .iter()
                    .any(|known| same_presence(known, &presence));
            if !twin {
                report.problems.push(Problem {
                    folder: label,
                    message: format!("{name} is already used by {winner} ({winner_folder})"),
                });
            }
            continue;
        }
        taken.insert(identity(&name), (name.clone(), label.clone()));
        for message in warnings {
            report.problems.push(Problem {
                folder: label.clone(),
                message,
            });
        }
        report.locales.insert(name.clone(), locales);
        report.folders.insert(name.clone(), folder);
        report.origins.insert(name, origin);
        report.presences.push(presence);
    }
    report
        .presences
        .sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(name: &str) -> String {
        format!(r#"{{"name":"{name}","version":"1","client_id":"12345678901234567"}}"#)
    }

    fn presence(root: &Path, folder: &str, name: &str, script: &str) {
        let directory = root.join(folder);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("manifest.json"), manifest(name)).unwrap();
        fs::write(directory.join("main.js"), script).unwrap();
    }

    #[test]
    fn presences_are_loaded_and_sorted_by_name() {
        let root = tempfile::tempdir().unwrap();
        presence(root.path(), "b", "Beta", "function read(){}");
        presence(root.path(), "a", "Alpha", "function read(){}");
        let report = load_presences(&[root.path().to_owned()]);
        let names: Vec<_> = report
            .presences
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect();
        assert_eq!(names, ["Alpha", "Beta"]);
        assert!(report.problems.is_empty());
        assert_eq!(report.presences[0].source, "function read(){}");
    }

    #[test]
    fn a_broken_folder_is_reported_and_the_others_still_load() {
        let root = tempfile::tempdir().unwrap();
        presence(root.path(), "good", "Good", "x");
        fs::create_dir(root.path().join("bad")).unwrap();
        fs::write(root.path().join("bad/manifest.json"), "{ nope").unwrap();
        fs::create_dir(root.path().join("noscript")).unwrap();
        fs::write(
            root.path().join("noscript/manifest.json"),
            manifest("NoScript"),
        )
        .unwrap();
        let report = load_presences(&[root.path().to_owned()]);
        assert_eq!(report.presences.len(), 1);
        let mut folders: Vec<_> = report.problems.iter().map(|p| p.folder.as_str()).collect();
        folders.sort();
        assert_eq!(folders, ["bad", "noscript"]);
    }

    #[test]
    fn hidden_folders_files_and_missing_roots_are_ignored() {
        let root = tempfile::tempdir().unwrap();
        presence(root.path(), ".git", "Hidden", "x");
        fs::write(root.path().join("notes.txt"), "x").unwrap();
        let missing = root.path().join("does-not-exist");
        let report = load_presences(&[root.path().to_owned(), missing]);
        assert!(report.presences.is_empty());
        assert!(report.problems.is_empty());
    }

    #[test]
    fn the_first_root_wins_when_names_repeat() {
        let user = tempfile::tempdir().unwrap();
        let bundled = tempfile::tempdir().unwrap();
        presence(user.path(), "mine", "Same", "user version");
        presence(bundled.path(), "stock", "Same", "bundled version");
        let report = load_presences(&[user.path().to_owned(), bundled.path().to_owned()]);
        assert_eq!(report.presences.len(), 1);
        assert_eq!(report.presences[0].source, "user version");
        assert_eq!(report.problems[0].folder, "stock");
    }

    #[test]
    fn a_refused_folder_is_reported_and_leaves_its_name_to_the_next_root() {
        let user = tempfile::tempdir().unwrap();
        let bundled = tempfile::tempdir().unwrap();
        presence(user.path(), "same", "Same", "edited copy");
        presence(user.path(), "other", "Other", "kept");
        presence(bundled.path(), "same", "Same", "bundled version");
        let refused = user.path().join("same");
        let report = load_presences_checked(
            &[user.path().to_owned(), bundled.path().to_owned()],
            &|folder| (folder == refused).then(|| "changed".to_owned()),
        );
        let sources: Vec<_> = report.presences.iter().map(|p| p.source.as_str()).collect();
        assert_eq!(sources, ["kept", "bundled version"]);
        assert_eq!(report.folders["Same"], bundled.path().join("same"));
        assert_eq!(report.problems.len(), 1);
        assert_eq!(report.problems[0].folder, "same");
        assert_eq!(report.problems[0].message, "changed");
    }

    /// Classifies by the root a folder sits in: `catalog` and `mine` folders by name.
    fn by_folder(folder: &Path) -> Result<Origin, String> {
        let name = folder
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        match name {
            n if n.starts_with("cat") => Ok(Origin::Catalog),
            n if n.starts_with("broken") => Err("changed after it was installed".into()),
            n if n.starts_with("stock") => Ok(Origin::Bundled),
            _ => Ok(Origin::Local),
        }
    }

    fn ranked(roots: &[&tempfile::TempDir]) -> LoadReport {
        let paths: Vec<PathBuf> = roots.iter().map(|root| root.path().to_owned()).collect();
        load_presences_ranked(&paths, &|name| std::env::var(name).ok(), &by_folder)
    }

    #[test]
    fn a_local_folder_never_takes_the_name_of_a_bundled_or_catalog_presence() {
        // The user's folder comes first and its folder name sorts first, and still loses.
        for roots_first in [true, false] {
            let user = tempfile::tempdir().unwrap();
            let other = tempfile::tempdir().unwrap();
            presence(user.path(), "aaa-mine", "YouTube", "local script");
            presence(user.path(), "cat-youtube", "Netflix", "catalog script");
            presence(other.path(), "stock-youtube", "YouTube", "bundled script");
            presence(other.path(), "aaa-local", "Netflix", "local script");
            let report = if roots_first {
                ranked(&[&user, &other])
            } else {
                ranked(&[&other, &user])
            };
            let by_name: HashMap<_, _> = report
                .presences
                .iter()
                .map(|p| (p.manifest.name.as_str(), p.source.as_str()))
                .collect();
            assert_eq!(by_name["YouTube"], "bundled script", "{roots_first}");
            assert_eq!(by_name["Netflix"], "catalog script", "{roots_first}");
            assert_eq!(report.origins["YouTube"], Origin::Bundled);
            assert_eq!(report.origins["Netflix"], Origin::Catalog);
            assert_eq!(report.problems.len(), 2);
            assert!(
                report
                    .problems
                    .iter()
                    .all(|p| p.message.contains("already used by"))
            );
        }
    }

    #[test]
    fn names_that_differ_only_in_case_or_separators_clash() {
        let root = tempfile::tempdir().unwrap();
        presence(root.path(), "stock-a", "My Site", "bundled");
        for (folder, name) in [
            ("mine-1", "my site"),
            ("mine-2", "MY_SITE"),
            ("mine-3", "my-site"),
        ] {
            presence(root.path(), folder, name, "local");
        }
        presence(root.path(), "mine-4", "MySite", "local");
        let report = ranked(&[&root]);
        let names: Vec<_> = report
            .presences
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect();
        assert_eq!(names, ["My Site", "MySite"]);
        assert_eq!(report.problems.len(), 3);
        let message = &report.problems[0].message;
        assert!(message.contains("My Site (stock-a)"), "{message}");
    }

    #[test]
    fn an_installed_copy_takes_priority_over_the_bundled_one() {
        let user = tempfile::tempdir().unwrap();
        let bundled = tempfile::tempdir().unwrap();
        presence(user.path(), "cat-demo", "Demo", "installed");
        presence(bundled.path(), "stock-demo", "Demo", "bundled");
        let report = ranked(&[&user, &bundled]);
        assert_eq!(report.presences[0].source, "installed");
        assert_eq!(report.origins["Demo"], Origin::Catalog);
    }

    #[test]
    fn of_two_local_folders_the_first_wins_and_the_problem_names_it() {
        let root = tempfile::tempdir().unwrap();
        presence(root.path(), "mine-a", "Same", "first");
        presence(root.path(), "mine-b", "same", "second");
        let report = ranked(&[&root]);
        assert_eq!(report.presences.len(), 1);
        assert_eq!(report.presences[0].source, "first");
        assert_eq!(report.problems[0].folder, "mine-b");
        assert!(report.problems[0].message.contains("Same (mine-a)"));
    }

    #[test]
    fn an_identical_copy_is_ignored_unless_it_is_a_local_folder() {
        let user = tempfile::tempdir().unwrap();
        let bundled = tempfile::tempdir().unwrap();
        presence(user.path(), "cat-demo", "Demo", "same script");
        presence(bundled.path(), "stock-demo", "Demo", "same script");
        assert!(ranked(&[&user, &bundled]).problems.is_empty());

        let user = tempfile::tempdir().unwrap();
        presence(user.path(), "mine-demo", "Demo", "same script");
        let report = ranked(&[&user, &bundled]);
        assert_eq!(report.origins["Demo"], Origin::Bundled);
        assert_eq!(report.problems.len(), 1, "a local lookalike is reported");
    }

    #[test]
    fn a_folder_the_classifier_refuses_is_not_loaded_and_says_why() {
        let user = tempfile::tempdir().unwrap();
        let bundled = tempfile::tempdir().unwrap();
        presence(user.path(), "broken-demo", "Demo", "edited by hand");
        presence(bundled.path(), "stock-demo", "Demo", "bundled");
        let report = ranked(&[&user, &bundled]);
        assert_eq!(report.presences[0].source, "bundled");
        assert_eq!(report.problems.len(), 1);
        assert_eq!(report.problems[0].folder, "broken-demo");
        assert!(report.problems[0].message.contains("changed after"));
        assert!(!report.folders.values().any(|f| f.ends_with("broken-demo")));
    }

    #[test]
    fn declared_folders_have_their_variables_expanded() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("game");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("manifest.json"),
            r#"{"name":"Game","version":"1","client_id":"12345678901234567",
                "permissions":{"files":["%LOCALAPPDATA%/Game/logs","%NOPE%/x"]}}"#,
        )
        .unwrap();
        fs::write(directory.join("main.js"), "x").unwrap();
        let lookup =
            |name: &str| (name == "LOCALAPPDATA").then(|| "C:/Users/me/AppData/Local".to_owned());
        let report = load_presences_with(&[root.path().to_owned()], &lookup);
        let files = &report.presences[0].manifest.permissions.files;
        assert_eq!(files, &["C:/Users/me/AppData/Local/Game/logs"]);
    }

    #[test]
    fn identical_copies_are_ignored_without_a_problem() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        presence(first.path(), "demo", "Same", "same script");
        presence(second.path(), "copy", "Same", "same script");
        let report = load_presences(&[first.path().to_owned(), second.path().to_owned()]);
        assert_eq!(report.presences.len(), 1);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
    }

    #[test]
    fn oversized_scripts_are_refused() {
        let root = tempfile::tempdir().unwrap();
        presence(
            root.path(),
            "big",
            "Big",
            &"x".repeat(MAX_SCRIPT as usize + 1),
        );
        let report = load_presences(&[root.path().to_owned()]);
        assert!(report.presences.is_empty());
        assert!(report.problems[0].message.contains("larger"));
    }

    #[test]
    fn an_entry_that_climbs_out_of_the_folder_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("sneaky");
        fs::create_dir_all(&folder).unwrap();
        fs::write(root.path().join("outside.js"), "x").unwrap();
        fs::write(
            folder.join("manifest.json"),
            r#"{"name":"Sneaky","version":"1","client_id":"12345678901234567","entry":"../outside.js"}"#,
        )
        .unwrap();
        let report = load_presences(&[root.path().to_owned()]);
        assert!(report.presences.is_empty());
        assert_eq!(report.problems.len(), 1);
    }

    #[test]
    fn the_bundled_presences_of_this_repository_all_load() {
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presences");
        let report = load_presences(&[bundled]);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let names: Vec<_> = report
            .presences
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect();
        assert!(
            names.contains(&"Roblox") && names.contains(&"YouTube"),
            "{names:?}"
        );
    }

    #[test]
    fn translations_are_loaded_and_bad_files_are_reported() {
        let root = tempfile::tempdir().unwrap();
        presence(root.path(), "a", "Alpha", "function read(){}");
        let locales = root.path().join("a").join("locales");
        fs::create_dir_all(&locales).unwrap();
        fs::write(locales.join("en.json"), r#"{"hi":"Hello"}"#).unwrap();
        fs::write(locales.join("es.json"), r#"{"hi":"Hola"}"#).unwrap();
        fs::write(locales.join("pt-BR.json"), r#"{"hi":"Oi"}"#).unwrap();
        fs::write(locales.join("broken.json"), "{").unwrap();
        fs::write(locales.join("es_ar.json"), r#"{"hi":"Che"}"#).unwrap();
        fs::write(locales.join("notes.txt"), "ignored").unwrap();
        fs::write(locales.join("fr.json"), r#"{"hi":1}"#).unwrap();
        let report = load_presences(&[root.path().to_owned()]);
        assert_eq!(report.presences.len(), 1, "the presence still loads");
        let tables = &report.locales["Alpha"];
        assert_eq!(
            tables.keys().map(String::as_str).collect::<Vec<_>>(),
            ["en", "es", "pt-BR"]
        );
        // broken.json is not a tag, es_ar.json is not in canonical form and
        // fr.json holds a value that is not text.
        assert_eq!(report.problems.len(), 3);
        assert_eq!(locale::resolve(tables, "es-AR")["hi"], "Hola");
    }

    #[test]
    fn each_presence_remembers_its_folder_and_one_folder_can_be_read_alone() {
        let root = tempfile::tempdir().unwrap();
        presence(root.path(), "a", "Alpha", "function read(){}");
        let locales = root.path().join("a").join("locales");
        fs::create_dir_all(&locales).unwrap();
        fs::write(locales.join("en.json"), r#"{"hi":"Hello"}"#).unwrap();

        let report = load_presences(&[root.path().to_owned()]);
        assert_eq!(report.folders["Alpha"], root.path().join("a"));

        let found = load_presence(&root.path().join("a")).unwrap();
        assert_eq!(found.presence.manifest.name, "Alpha");
        assert_eq!(found.locales["en"]["hi"], "Hello");
        assert!(found.warnings.is_empty());
        assert!(load_presence(&root.path().join("missing")).is_err());
    }
}
