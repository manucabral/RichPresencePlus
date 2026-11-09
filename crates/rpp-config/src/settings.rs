//! User settings, written atomically.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rpp_core::Preferences;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::manual::Manual;
use crate::profiles::Profile;

/// Port the browser extension is paired with unless the user changes it.
pub const DEFAULT_PORT: u16 = 38417;
/// Log detail until the user changes it.
pub const DEFAULT_LOG_LEVEL: &str = "info";
/// Most presences a user can mark as favorite.
pub const MAX_FAVORITES: usize = 500;

/// The log levels a user can choose, from least to most detail.
pub const LOG_LEVELS: [&str; 4] = ["error", "warn", "info", "debug"];

/// Largest settings file that is read.
const MAX_BYTES: u64 = 1024 * 1024;

/// Layout of `settings.json` this build writes.
pub const SCHEMA_VERSION: u32 = 1;

/// A step that turns a file of one version into the next.
type Migration = fn(&mut Map<String, Value>);

/// Files written before the version was recorded have the layout of version 1.
fn from_unversioned(_: &mut Map<String, Value>) {}

/// Entry `n` turns version `n` into `n + 1`. A change to the layout adds a
/// step here and bumps [`SCHEMA_VERSION`]; a step never removes what a user chose.
const MIGRATIONS: [Migration; SCHEMA_VERSION as usize] = [from_unversioned];

/// How a file compared with this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Age {
    Current,
    Older,
    /// Written by a newer build; fields this build does not know are lost on save.
    Newer,
}

/// Brings a settings document up to the version the steps lead to.
///
/// Raises:
///     String: The document is not an object or its version is not a number.
fn migrate(document: &mut Value, steps: &[Migration]) -> Result<Age, String> {
    let object = document
        .as_object_mut()
        .ok_or("the file does not hold an object")?;
    let version = match object.get("schema_version") {
        None => 0,
        Some(value) => value
            .as_u64()
            .ok_or("schema_version is not a whole number")?,
    };
    let target = steps.len() as u64;
    let age = match version.cmp(&target) {
        std::cmp::Ordering::Equal => Age::Current,
        std::cmp::Ordering::Greater => Age::Newer,
        std::cmp::Ordering::Less => {
            for step in &steps[version as usize..] {
                step(object);
            }
            Age::Older
        }
    };
    object.insert("schema_version".into(), Value::from(target));
    Ok(age)
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

/// Everything the user can change and the application remembers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Layout version of the file; see [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Publishing rules.
    pub preferences: Preferences,
    /// Names of the presences the user turned off.
    pub disabled: BTreeSet<String>,
    /// Names of the presences the user marked as favorite, installed or not.
    pub favorites: BTreeSet<String>,
    /// Loopback port of the extension bridge.
    pub bridge_port: u16,
    /// Secret the extension must present; created on first use.
    pub bridge_token: Option<String>,
    /// Language of the interface and presences: `auto` or a tag such as `es`.
    pub language: String,
    /// Values picked for the options each presence declares, by presence and key.
    pub options: BTreeMap<String, BTreeMap<String, Value>>,
    /// Hosts the user added to presences that read sites of one's own, by presence.
    pub sites: BTreeMap<String, Vec<String>>,
    /// Detail of the log file: `error`, `warn`, `info` or `debug`.
    pub log_level: String,
    /// Activities written by hand and the one on display.
    pub manual: Manual,
    /// Named sets of publishing rules the user can switch between.
    pub profiles: Vec<Profile>,
    /// Author written into the presences the user creates or copies.
    pub author: Option<String>,
    /// Minimizing the window hides it in the tray instead of the taskbar.
    pub minimize_to_tray: bool,
    /// The user answered whether to start with Windows; the question is not asked again.
    pub startup_asked: bool,
    /// Which Discord client receives the activity when several run.
    pub discord_client: rpp_discord::DiscordClient,
    /// Keyboard shortcut that pauses or resumes publishing; off when unset.
    pub pause_shortcut: Option<String>,
    /// Count the time each presence stays on display, on this computer only.
    pub track_time: bool,
    /// Answer overlays and stream decks on the loopback.
    pub local_api: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            preferences: Preferences::default(),
            disabled: BTreeSet::new(),
            favorites: BTreeSet::new(),
            bridge_port: default_port(),
            bridge_token: None,
            language: "auto".to_owned(),
            options: BTreeMap::new(),
            sites: BTreeMap::new(),
            log_level: DEFAULT_LOG_LEVEL.to_owned(),
            manual: Manual::default(),
            profiles: Vec::new(),
            author: None,
            minimize_to_tray: true,
            startup_asked: false,
            discord_client: rpp_discord::DiscordClient::Auto,
            pause_shortcut: None,
            track_time: false,
            local_api: false,
        }
    }
}

impl Settings {
    /// The 64-character bridge token, created and kept if missing.
    pub fn token(&mut self) -> &str {
        self.bridge_token
            .get_or_insert_with(rpp_bridge::generate_token)
    }

    /// The pairing code the extension asks for, as `port:token`.
    pub fn pairing_code(&mut self) -> String {
        let port = self.bridge_port;
        format!("{port}:{}", self.token())
    }
}

/// The result of reading the settings file.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// The settings to use.
    pub settings: Settings,
    /// Set when the file could not be used; the user should be told.
    pub warning: Option<String>,
}

/// Reads and writes `settings.json` in one folder.
#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
    /// A file this build cannot fully read has no copy, so saving would lose it.
    keep_file: Arc<AtomicBool>,
}

impl SettingsStore {
    /// Creates a store for the `settings.json` in `directory`.
    pub fn new(directory: &Path) -> Self {
        Self {
            path: directory.join("settings.json"),
            keep_file: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Copies the file aside as `name` before a save can replace it.
    ///
    /// Returns:
    ///     The end of the warning that tells what happened to the old file.
    fn set_aside(&self, name: &str) -> String {
        let kept = fs::copy(&self.path, self.path.with_file_name(name)).is_ok();
        self.keep_file.store(!kept, Ordering::Relaxed);
        if kept {
            format!("A copy was kept as {name}.")
        } else {
            "No copy could be made, so changes will not be saved.".to_owned()
        }
    }

    /// Reads the settings.
    pub fn load(&self) -> Loaded {
        self.keep_file.store(false, Ordering::Relaxed);
        let text = match fs::metadata(&self.path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Loaded {
                    settings: Settings::default(),
                    warning: None,
                };
            }
            Ok(meta) if meta.len() > MAX_BYTES => Err("the file is larger than 1 MiB".to_owned()),
            _ => fs::read_to_string(&self.path).map_err(|e| e.to_string()),
        };
        let mut age = Age::Current;
        let parsed = text.and_then(|text| {
            let mut document: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            age = migrate(&mut document, &MIGRATIONS)?;
            let mut settings: Settings =
                serde_json::from_value(document).map_err(|e| e.to_string())?;
            settings.preferences = settings
                .preferences
                .validated()
                .map_err(|e| e.to_string())?;
            if settings.bridge_port == 0 {
                return Err("the bridge port must be above zero".to_owned());
            }
            if settings.manual.shown().is_none() {
                settings.manual.active = None;
            }
            // A shortcut edited into something unusable is turned off.
            settings.pause_shortcut = settings
                .pause_shortcut
                .take()
                .and_then(|shortcut| crate::shortcut::normalize(&shortcut).ok());
            // A rotation edited into something invalid is dropped, not guessed.
            settings.manual.rotation = settings
                .manual
                .rotation
                .take()
                .and_then(|rotation| rotation.validated(&settings.manual.presets).ok());
            if settings
                .author
                .as_deref()
                .is_some_and(|author| !rpp_plugins::valid_person(author))
            {
                settings.author = None;
            }
            // A file edited by hand keeps only names a presence could have,
            // up to the limit.
            settings.favorites = std::mem::take(&mut settings.favorites)
                .into_iter()
                .filter(|name| rpp_plugins::valid_name(name))
                .take(MAX_FAVORITES)
                .collect();
            if !LOG_LEVELS.contains(&settings.log_level.as_str()) {
                // Not worth pausing publishing over: the default is safe.
                settings.log_level = DEFAULT_LOG_LEVEL.to_owned();
            }
            Ok(settings)
        });
        match parsed {
            Ok(settings) if age == Age::Newer => {
                let kept = self.set_aside("settings.newer.json");
                Loaded {
                    settings,
                    warning: Some(format!(
                        "settings.json was written by a newer version of the application. {kept}"
                    )),
                }
            }
            Ok(settings) => Loaded {
                settings,
                warning: None,
            },
            Err(reason) => {
                let kept = self.set_aside("settings.invalid.json");
                let mut settings = Settings::default();
                settings.preferences.global.paused = true;
                Loaded {
                    settings,
                    warning: Some(format!(
                        "Publishing is paused: settings.json could not be read ({reason}). {kept}"
                    )),
                }
            }
        }
    }

    /// Writes the settings so that a crash never leaves a partial file.
    pub fn save(&self, settings: &Settings) -> io::Result<()> {
        if self.keep_file.load(Ordering::Relaxed) {
            return Err(io::Error::other(
                "settings.json was kept: no copy of it could be made",
            ));
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = self.path.with_file_name("settings.json.tmp");
        let mut file = File::create(&temporary)?;
        file.write_all(&serde_json::to_vec_pretty(settings)?)?;
        file.sync_all()?;
        drop(file);
        fs::rename(temporary, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SettingsStore) {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(directory.path());
        (directory, store)
    }

    #[test]
    fn favorites_from_a_hand_edited_file_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        let many: Vec<String> = (0..MAX_FAVORITES + 20).map(|n| format!("P{n}")).collect();
        let mut names = many.clone();
        names.push(String::new());
        names.push("x".repeat(500));
        let text = serde_json::json!({ "favorites": names }).to_string();
        std::fs::write(dir.path().join("settings.json"), text).unwrap();
        let loaded = SettingsStore::new(dir.path()).load();
        assert!(loaded.warning.is_none(), "{:?}", loaded.warning);
        let favorites = loaded.settings.favorites;
        assert_eq!(favorites.len(), MAX_FAVORITES);
        assert!(favorites.iter().all(|name| rpp_plugins::valid_name(name)));
    }

    #[test]
    fn a_missing_file_gives_defaults_without_a_warning() {
        let (_directory, store) = store();
        let loaded = store.load();
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.warning, None);
        assert_eq!(loaded.settings.bridge_port, DEFAULT_PORT);
    }

    #[test]
    fn settings_survive_a_save_and_load() {
        let (_directory, store) = store();
        let mut settings = Settings::default();
        settings.disabled.insert("Roblox".into());
        settings.preferences.global.excluded_domains = vec!["example.com".into()];
        let token = settings.token().to_owned();
        store.save(&settings).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.settings, settings);
        assert_eq!(
            loaded.settings.bridge_token.as_deref(),
            Some(token.as_str())
        );
        assert_eq!(loaded.warning, None);
    }

    #[test]
    fn the_token_is_created_once_and_kept() {
        let mut settings = Settings::default();
        let first = settings.token().to_owned();
        assert_eq!(first.len(), 64);
        assert_eq!(settings.token(), first);
        assert_eq!(settings.pairing_code(), format!("{DEFAULT_PORT}:{first}"));
    }

    #[test]
    fn unknown_fields_in_an_older_file_are_tolerated_and_new_ones_default() {
        let (directory, store) = store();
        fs::write(
            directory.path().join("settings.json"),
            r#"{"disabled": ["A"], "future_option": true}"#,
        )
        .unwrap();
        let loaded = store.load();
        assert_eq!(loaded.warning, None);
        assert!(loaded.settings.disabled.contains("A"));
        assert_eq!(loaded.settings.bridge_port, DEFAULT_PORT);
    }

    #[test]
    fn an_unreadable_file_is_kept_and_publishing_starts_paused() {
        let (directory, store) = store();
        fs::write(directory.path().join("settings.json"), "{ not json").unwrap();
        let loaded = store.load();
        assert!(loaded.settings.preferences.global.paused);
        assert!(loaded.warning.unwrap().contains("settings.invalid.json"));
        assert_eq!(
            fs::read_to_string(directory.path().join("settings.invalid.json")).unwrap(),
            "{ not json"
        );
    }

    #[test]
    fn a_file_with_no_copy_is_not_replaced_and_the_warning_says_so() {
        let (directory, store) = store();
        let settings = directory.path().join("settings.json");
        fs::write(&settings, "{ not json").unwrap();
        // A folder with the name of the copy makes the copy fail.
        fs::create_dir(directory.path().join("settings.invalid.json")).unwrap();

        let loaded = store.load();
        assert!(loaded.warning.unwrap().contains("No copy could be made"));
        assert!(store.save(&loaded.settings).is_err());
        assert_eq!(fs::read_to_string(&settings).unwrap(), "{ not json");

        // Once a copy can be made, saving works again.
        fs::remove_dir(directory.path().join("settings.invalid.json")).unwrap();
        let loaded = store.load();
        assert!(store.save(&loaded.settings).is_ok());
    }

    #[test]
    fn invalid_preferences_are_treated_like_an_unreadable_file() {
        let (directory, store) = store();
        fs::write(
            directory.path().join("settings.json"),
            r#"{"preferences": {"global": {"excluded_domains": ["https://a.com/x"]}}}"#,
        )
        .unwrap();
        let loaded = store.load();
        assert!(loaded.settings.preferences.global.paused);
        assert!(loaded.warning.is_some());
    }

    #[test]
    fn a_shown_preset_that_no_longer_exists_is_forgotten_without_pausing() {
        let (_directory, store) = store();
        let mut settings = Settings::default();
        settings.manual.active = Some("gone".into());
        store.save(&settings).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.settings.manual.active, None);
        assert_eq!(loaded.warning, None);
        assert!(!loaded.settings.preferences.global.paused);
    }

    #[test]
    fn a_file_from_before_versions_is_read_and_saved_with_the_current_version() {
        let (directory, store) = store();
        let path = directory.path().join("settings.json");
        fs::write(&path, r#"{"disabled": ["A"], "log_level": "debug"}"#).unwrap();
        let loaded = store.load();
        assert_eq!(loaded.warning, None);
        assert_eq!(loaded.settings.schema_version, SCHEMA_VERSION);
        assert!(loaded.settings.disabled.contains("A"));
        assert_eq!(loaded.settings.log_level, "debug");
        store.save(&loaded.settings).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["schema_version"], SCHEMA_VERSION);
    }

    #[test]
    fn migrations_run_in_order_from_the_version_of_the_file() {
        fn rename(object: &mut Map<String, Value>) {
            if let Some(value) = object.remove("old_name") {
                object.insert("middle_name".into(), value);
            }
        }
        fn rename_again(object: &mut Map<String, Value>) {
            if let Some(value) = object.remove("middle_name") {
                object.insert("new_name".into(), value);
            }
        }
        let steps: [Migration; 3] = [from_unversioned, rename, rename_again];

        let mut oldest = serde_json::json!({"old_name": 7});
        assert_eq!(migrate(&mut oldest, &steps), Ok(Age::Older));
        assert_eq!(
            oldest,
            serde_json::json!({"new_name": 7, "schema_version": 3})
        );

        // A file already at version 2 skips the first two steps.
        let mut middle = serde_json::json!({"schema_version": 2, "middle_name": 1, "old_name": 9});
        migrate(&mut middle, &steps).unwrap();
        assert_eq!(middle["new_name"], 1);
        assert_eq!(
            middle["old_name"], 9,
            "a step that already ran is not run again"
        );

        let mut current = serde_json::json!({"schema_version": 3});
        assert_eq!(migrate(&mut current, &steps), Ok(Age::Current));
        let mut bad = serde_json::json!({"schema_version": "two"});
        assert!(migrate(&mut bad, &steps).is_err());
        assert!(migrate(&mut serde_json::json!([1]), &steps).is_err());
    }

    #[test]
    fn a_file_from_a_newer_version_is_used_and_a_copy_kept_without_pausing() {
        let (directory, store) = store();
        let path = directory.path().join("settings.json");
        let newer = r#"{"schema_version": 99, "disabled": ["A"], "future_option": true}"#;
        fs::write(&path, newer).unwrap();
        let loaded = store.load();
        assert!(loaded.warning.unwrap().contains("settings.newer.json"));
        assert!(!loaded.settings.preferences.global.paused);
        assert!(loaded.settings.disabled.contains("A"));
        assert_eq!(loaded.settings.schema_version, SCHEMA_VERSION);
        assert_eq!(
            fs::read_to_string(directory.path().join("settings.newer.json")).unwrap(),
            newer
        );
    }

    #[test]
    fn a_version_that_is_not_a_number_makes_the_file_unusable() {
        let (directory, store) = store();
        fs::write(
            directory.path().join("settings.json"),
            r#"{"schema_version": "one"}"#,
        )
        .unwrap();
        let loaded = store.load();
        assert!(loaded.settings.preferences.global.paused);
        assert!(loaded.warning.unwrap().contains("settings.invalid.json"));
    }

    #[test]
    fn window_choices_default_to_the_tray_and_a_question_not_yet_asked() {
        let (directory, store) = store();
        fs::write(
            directory.path().join("settings.json"),
            r#"{"disabled": ["A"]}"#,
        )
        .unwrap();
        let loaded = store.load();
        assert!(loaded.settings.minimize_to_tray);
        assert!(!loaded.settings.startup_asked);

        let mut settings = loaded.settings;
        settings.minimize_to_tray = false;
        settings.startup_asked = true;
        store.save(&settings).unwrap();
        let again = store.load().settings;
        assert!(!again.minimize_to_tray && again.startup_asked);
    }

    #[test]
    fn saving_leaves_no_temporary_file() {
        let (directory, store) = store();
        store.save(&Settings::default()).unwrap();
        store.save(&Settings::default()).unwrap();
        let names: Vec<_> = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["settings.json"]);
    }
}
