//! The controller and the way it starts.

use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rpp_bridge::{Bridge, Config, Rule};
use rpp_catalog::Catalog;
use rpp_config::{LOG_LEVELS, Origin, Settings, SettingsStore};
use rpp_core::Preferences;
use rpp_core::locale;
use rpp_core::locale::Table;
use rpp_discord::Connector;
use rpp_plugins::Setting;
use rpp_plugins::{Host, Request};
use rpp_service::Service;
use serde_json::{Map, Value};

use crate::develop::{Reloads, Watched};
use crate::library::{Library, effective_language, options_of};
use crate::views::{
    Access, AuthorView, ExtensionSessions, Glance, LanguageView, OptionView, Overview, Pairing,
    PresenceView,
};

/// The options of a presence as the window shows them.
fn option_views(
    declared: &[Setting],
    values: &Map<String, Value>,
    strings: &Table,
) -> Vec<OptionView> {
    let text = |key: String, fallback: &str| {
        strings
            .get(&key)
            .cloned()
            .unwrap_or_else(|| fallback.to_owned())
    };
    declared
        .iter()
        .map(|setting| {
            let key = setting.key();
            let choice_labels = match setting {
                Setting::Choice { choices, .. } => choices
                    .iter()
                    .map(|choice| text(format!("option.{key}.{choice}"), choice))
                    .collect(),
                _ => Vec::new(),
            };
            OptionView {
                setting: setting.clone(),
                label: text(format!("option.{key}"), key),
                value: values
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| setting.default_value()),
                choice_labels,
                help: strings.get(&format!("help.{key}")).cloned(),
            }
        })
        .collect()
}

/// How long the answer to "is Discord running" is reused for the window.
const DISCORD_CACHE: Duration = Duration::from_secs(2);
/// How often the programs are listed for the quiet rule. Each list costs about
/// 15 ms of a core, and nobody without the rule pays for it.
const QUIET_LIST_EVERY: Duration = Duration::from_secs(15);
/// How long the tray reuses it. It only paints a dot, and listing every process
/// of Windows takes about 15 ms: asked every 2 seconds for good, that was nearly a
/// percent of a core with the window closed.
const TRAY_DISCORD_CACHE: Duration = Duration::from_secs(30);

/// Where things live and how the application identifies itself.
#[derive(Debug, Clone)]
pub struct Options {
    /// Folder for settings.
    pub data_dir: PathBuf,
    /// Folders that hold presences, in priority order.
    pub presence_roots: Vec<PathBuf>,
    /// Version shown to the user.
    pub version: String,
    /// Replaces the saved bridge port; 0 picks a free one. Used by tests.
    pub port_override: Option<u16>,
    /// Language of the system, used while the setting is `auto`.
    pub system_language: String,
    /// The signed catalog of approved presences, when the application has one.
    pub catalog: Option<Arc<Catalog>>,
}

/// Why the application could not start.
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// The settings could not be written.
    #[error("cannot write settings: {0}")]
    Settings(#[from] io::Error),
}

/// The running application.
pub struct Controller<H> {
    pub(crate) store: SettingsStore,
    pub(crate) settings: Mutex<Settings>,
    pub(crate) library: Mutex<Library>,
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) data_dir: PathBuf,
    pub(crate) catalog: Option<Arc<Catalog>>,
    pub(crate) system_language: String,
    pub(crate) discord_seen: Mutex<Option<(Instant, bool)>>,
    /// The last list of programs for the quiet rule, and when it was read.
    programs_seen: Mutex<Option<(Instant, Vec<String>)>>,
    /// What the presences folder held at the last reading.
    pub(crate) folders_seen: Mutex<crate::folders::FolderState>,
    /// The time each presence stayed on display.
    pub(crate) tracker: Mutex<crate::usage::Tracker>,
    /// The local address, while it is on.
    pub(crate) local_api: Mutex<Option<crate::local_api::Running>>,
    /// The port of the local address.
    pub(crate) api_port: u16,
    /// Where the local address runs; the calls that start it come from any thread.
    pub(crate) runtime: tokio::runtime::Handle,
    pub(crate) warning: Option<String>,
    pub(crate) host: H,
    pub(crate) service: Service<H>,
    pub(crate) bridge: Bridge,
    pub(crate) version: String,
    pub(crate) manual_since: Mutex<u64>,
    pub(crate) watched: Mutex<Option<Watched>>,
    pub(crate) reloads: Mutex<Reloads>,
    /// Held while the developer tools write, so two saves cannot take one number.
    pub(crate) workbench: Mutex<()>,
    /// Time without input at the last reading, kept so a change of the
    /// setting applies at once instead of at the next reading.
    pub(crate) idle_for: Mutex<Option<Duration>>,
    /// Unix second the current rotation of custom statuses began.
    pub(crate) rotation_since: Mutex<u64>,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// Loads settings and presences, opens the bridge and starts publishing.
    ///
    /// Raises:
    ///     StartError: The first settings file cannot be written.
    pub async fn start<C>(options: Options, connector: C, host: H) -> Result<Self, StartError>
    where
        C: Connector + Clone + Send + 'static,
    {
        let store = SettingsStore::new(&options.data_dir);
        let loaded = store.load();
        let mut settings = loaded.settings;
        let mut warning = loaded.warning;
        let had_token = settings.bridge_token.is_some();
        let token = settings.token().to_owned();
        if !had_token && warning.is_none() {
            store.save(&settings)?;
        }

        let wanted = options.port_override.unwrap_or(settings.bridge_port);
        let bridge = match Bridge::start(Config::new(wanted, token.clone())).await {
            Ok(bridge) => bridge,
            Err(error) => {
                let bridge = Bridge::start(Config::new(0, token)).await?;
                warning.get_or_insert_with(|| {
                    format!(
                        "Port {wanted} is not available ({error}); using port {}. \
                         The browser extension finds it by itself; if you paired it \
                         with a code, paste the new one.",
                        bridge.address().port()
                    )
                });
                bridge
            }
        };

        // The native messaging host reads this to tell the extension where to connect.
        let pairing = rpp_bridge::native::Pairing {
            port: bridge.address().port(),
            token: bridge.token().to_owned(),
        };
        if let Err(error) = rpp_bridge::native::save_pairing(&options.data_dir, &pairing) {
            tracing::warn!("cannot record the pairing for the extension: {error}");
        }

        let library = Library::load(&options.presence_roots, options.catalog.as_deref());
        tracing::info!(
            version = %options.version,
            presences = library.presences.len(),
            "Rich Presence Plus started"
        );
        for problem in &library.problems {
            tracing::warn!(folder = %problem.folder, "{}", problem.message);
        }
        if let Some(warning) = &warning {
            tracing::warn!("{warning}");
        }
        let language = effective_language(&settings, &options.system_language);
        let enabled = library.enabled(&settings, &language);
        bridge.set_rules(Rule::from_manifests(enabled.iter().map(|p| &p.manifest)));
        let service = Service::start(
            connector,
            host.clone(),
            enabled,
            Arc::new(bridge.page_source()),
            settings.preferences.clone(),
        );
        bridge.on_page_changes(service.waker());
        let folders_seen = options
            .presence_roots
            .first()
            .map(|root| crate::folders::state_of(root))
            .unwrap_or_default();
        let controller = Self {
            store,
            settings: Mutex::new(settings),
            library: Mutex::new(library),
            roots: options.presence_roots,
            data_dir: options.data_dir,
            catalog: options.catalog,
            system_language: options.system_language,
            discord_seen: Mutex::new(None),
            programs_seen: Mutex::new(None),
            folders_seen: Mutex::new(folders_seen),
            tracker: Mutex::default(),
            local_api: Mutex::new(None),
            api_port: options
                .port_override
                .unwrap_or(crate::local_api::DEFAULT_PORT),
            runtime: tokio::runtime::Handle::current(),
            warning,
            host,
            service,
            bridge,
            version: options.version,
            manual_since: Mutex::new(0),
            watched: Mutex::new(None),
            reloads: Mutex::new(Reloads::default()),
            workbench: Mutex::new(()),
            idle_for: Mutex::new(None),
            rotation_since: Mutex::new(0),
        };
        controller.offer_saved_manual();
        if lock(&controller.settings).local_api
            && let Err(error) = controller.start_local_api()
        {
            tracing::warn!("the local address could not start: {error}");
        }
        Ok(controller)
    }

    /// Tells whether a Discord client is running.
    pub fn discord_running(&self) -> bool {
        self.discord_running_within(DISCORD_CACHE)
    }

    /// Like [`Self::discord_running`], with a reading up to `max_age` old.
    fn discord_running_within(&self, max_age: Duration) -> bool {
        let mut cached = lock(&self.discord_seen);
        if let Some((at, running)) = *cached
            && at.elapsed() < max_age
        {
            return running;
        }
        let choice = lock(&self.settings).discord_client;
        let running = match self.host.call(Request::ProcessList) {
            Ok(Value::Array(names)) => names
                .iter()
                .filter_map(Value::as_str)
                .any(|name| choice.serves(name)),
            _ => false,
        };
        *cached = Some((Instant::now(), running));
        running
    }

    /// Returns what the tray icon paints, which is a fraction of the overview.
    pub fn glance(&self) -> Glance {
        let status = self.service.status();
        let (paused, language) = {
            let settings = lock(&self.settings);
            let language = if settings.language == "auto" {
                self.system_language.clone()
            } else {
                settings.language.clone()
            };
            (settings.preferences.global.paused, language)
        };
        Glance {
            language,
            paused,
            discord_running: self.discord_running_within(TRAY_DISCORD_CACHE),
            idle: status
                .rows
                .iter()
                .any(|row| row.reason == rpp_core::policy::IDLE_REASON),
            quiet: status
                .rows
                .iter()
                .any(|row| row.reason == rpp_core::policy::QUIET_REASON),
            showing: status
                .rows
                .iter()
                .find(|row| row.selected)
                .map(|row| row.label.clone()),
        }
    }

    /// Returns what the dashboard shows.
    pub fn overview(&self) -> Overview {
        Overview {
            version: self.version.clone(),
            discord_running: self.discord_running(),
            extensions: self.bridge.extensions(),
            browsers: self.bridge.browsers(),
            discord_user: self.discord_user(),
            paused: lock(&self.settings).preferences.global.paused,
            warning: self.warning.clone(),
            status: self.service.status(),
            problems: lock(&self.library).problems.clone(),
        }
    }

    /// Returns every presence found, with its state.
    pub fn presences(&self) -> Vec<PresenceView> {
        let settings = lock(&self.settings);
        let language = effective_language(&settings, &self.system_language);
        let running = self.service.running();
        let library = lock(&self.library);
        library
            .presences
            .iter()
            .map(|presence| {
                let manifest = &presence.manifest;
                let strings = library.strings(&manifest.name, &language);
                let values = options_of(presence, &settings);
                let permissions = &manifest.permissions;
                PresenceView {
                    name: manifest.name.clone(),
                    version: manifest.version.clone(),
                    description: strings
                        .get("description")
                        .cloned()
                        .or_else(|| manifest.description.clone()),
                    kind: manifest.kind,
                    interval: manifest.interval,
                    hide_when_paused: manifest.hide_when_paused,
                    enabled: !settings.disabled.contains(&manifest.name),
                    running: running.contains(&manifest.name),
                    access: Access {
                        http: permissions.http.clone(),
                        process: permissions.process,
                        args: permissions.args.clone(),
                        window: permissions.window,
                        media: permissions.media,
                        query: permissions.query.clone(),
                        files: permissions.files.clone(),
                        sites: manifest
                            .page
                            .iter()
                            .flat_map(|page| page.sites().cloned())
                            .collect(),
                    },
                    options: option_views(&manifest.settings, &values, &strings),
                    origin: library.origin_of(&manifest.name).unwrap_or(Origin::Bundled),
                    editable: library.origin_of(&manifest.name) == Some(Origin::Local),
                    author: manifest.author.clone(),
                    color: manifest.color.clone(),
                    tags: manifest.tags.clone(),
                    icon: manifest
                        .icon
                        .clone()
                        .filter(|icon| icon.starts_with("https://")),
                }
            })
            .collect()
    }

    /// Saves the value of one option and hands it to the running script.
    ///
    /// Raises:
    ///     String: The presence or option does not exist, the value does not
    ///         fit the option, or the choice cannot be saved.
    pub fn set_option(&self, name: &str, key: &str, value: Value) -> Result<(), String> {
        let values = {
            let mut settings = lock(&self.settings);
            let library = lock(&self.library);
            let presence = library
                .find(name)
                .ok_or_else(|| format!("Unknown presence: {name}"))?;
            let setting = presence
                .manifest
                .settings
                .iter()
                .find(|setting| setting.key() == key)
                .ok_or_else(|| format!("{name} has no option {key}"))?;
            let clean = setting.check(&value)?;
            let mut changed = settings.clone();
            changed
                .options
                .entry(name.to_owned())
                .or_default()
                .insert(key.to_owned(), clean);
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
            options_of(presence, &settings)
        };
        self.service.set_settings(name, values);
        Ok(())
    }

    /// The logos kept as files in the presence folders, as `data:` images by name.
    pub fn icons(&self) -> std::collections::HashMap<String, String> {
        lock(&self.library).local_icons().clone()
    }

    /// Turns a presence on or off and remembers the choice.
    pub fn set_enabled(&self, name: &str, enabled: bool) -> Result<(), String> {
        {
            // Settings, then library: the one order every method takes them in.
            let mut settings = lock(&self.settings);
            if lock(&self.library).find(name).is_none() {
                return Err(format!("Unknown presence: {name}"));
            }
            let mut changed = settings.clone();
            if enabled {
                changed.disabled.remove(name);
            } else {
                changed.disabled.insert(name.to_owned());
            }
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
        }
        self.sync_running();
        Ok(())
    }

    /// Returns the shortcut that pauses publishing, if one is set.
    pub fn pause_shortcut(&self) -> Option<String> {
        lock(&self.settings).pause_shortcut.clone()
    }

    /// Saves the shortcut that pauses publishing; `None` turns it off.
    pub fn set_pause_shortcut(&self, shortcut: Option<&str>) -> Result<Option<String>, String> {
        let clean = shortcut.map(rpp_config::shortcut::normalize).transpose()?;
        let mut settings = lock(&self.settings);
        let mut changed = settings.clone();
        changed.pause_shortcut = clean.clone();
        self.store.save(&changed).map_err(|e| e.to_string())?;
        *settings = changed;
        Ok(clean)
    }

    /// Pauses publishing, or resumes it.
    pub fn toggle_pause(&self) -> Result<bool, String> {
        let mut preferences = self.preferences();
        preferences.global.paused = !preferences.global.paused;
        let paused = preferences.global.paused;
        self.set_preferences(preferences)?;
        Ok(paused)
    }

    /// Returns which Discord client receives the activity.
    pub fn discord_client(&self) -> rpp_discord::DiscordClient {
        lock(&self.settings).discord_client
    }

    /// Saves which Discord client receives the activity and moves it there.
    ///
    /// The connector must follow the choice before this reconnects.
    pub fn set_discord_client(&self, choice: rpp_discord::DiscordClient) -> Result<(), String> {
        {
            let mut settings = lock(&self.settings);
            let mut changed = settings.clone();
            changed.discord_client = choice;
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
        }
        // The answer about a running client depends on the choice.
        *lock(&self.discord_seen) = None;
        self.service.reconnect();
        Ok(())
    }

    /// Returns the names the user marked as favorite, sorted.
    pub fn favorites(&self) -> Vec<String> {
        lock(&self.settings).favorites.iter().cloned().collect()
    }

    /// Marks a presence as favorite or not; it does not have to be installed.
    ///
    /// Raises:
    ///     String: The name is not one a presence could have, the list is
    ///         full, or the settings cannot be saved.
    pub fn set_favorite(&self, name: &str, favorite: bool) -> Result<(), String> {
        if !rpp_plugins::valid_name(name) {
            return Err("That is not a presence name.".into());
        }
        let mut settings = lock(&self.settings);
        let mut changed = settings.clone();
        if favorite {
            if changed.favorites.len() >= rpp_config::MAX_FAVORITES
                && !changed.favorites.contains(name)
            {
                return Err(format!(
                    "You can keep up to {} favorites.",
                    rpp_config::MAX_FAVORITES
                ));
            }
            changed.favorites.insert(name.to_owned());
        } else {
            changed.favorites.remove(name);
        }
        self.store.save(&changed).map_err(|e| e.to_string())?;
        *settings = changed;
        Ok(())
    }

    /// Returns the saved language (`auto` or a tag) and the system language.
    pub fn language(&self) -> LanguageView {
        LanguageView {
            setting: lock(&self.settings).language.clone(),
            system: self.system_language.clone(),
        }
    }

    /// Saves the language, `auto` or a tag such as `pt-BR`, and restarts the
    /// running presences with its texts.
    pub fn set_language(&self, tag: &str) -> Result<(), String> {
        let tag = if tag == "auto" {
            tag.to_owned()
        } else {
            locale::normalize(tag).ok_or_else(|| format!("Not a language tag: {tag}"))?
        };
        let restart = {
            let mut settings = lock(&self.settings);
            if settings.language == tag {
                return Ok(());
            }
            let mut changed = settings.clone();
            changed.language = tag;
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
            let language = effective_language(&settings, &self.system_language);
            lock(&self.library).enabled(&settings, &language)
        };
        for presence in restart {
            self.service.start_presence(presence);
        }
        Ok(())
    }

    /// The author saved by the user and the Discord user name, if known.
    pub fn author(&self) -> AuthorView {
        AuthorView {
            saved: lock(&self.settings).author.clone(),
            discord: self.discord_user(),
        }
    }

    /// The author written into new presences: the saved one, else the Discord user name.
    pub(crate) fn default_author(&self) -> Option<String> {
        lock(&self.settings)
            .author
            .clone()
            .or_else(|| self.discord_user())
    }

    fn discord_user(&self) -> Option<String> {
        self.service
            .status()
            .discord_user
            .filter(|user| rpp_plugins::valid_person(user))
    }

    /// Saves the author written into new presences; empty text clears it.
    ///
    /// Raises:
    ///     String: The name is longer than 64 characters, holds control
    ///         characters or cannot be saved.
    pub fn set_author(&self, author: &str) -> Result<(), String> {
        let author = author.trim();
        let value = if author.is_empty() {
            None
        } else if rpp_plugins::valid_person(author) {
            Some(author.to_owned())
        } else {
            return Err("Use up to 64 characters of plain text for the author.".into());
        };
        let mut settings = lock(&self.settings);
        let mut changed = settings.clone();
        changed.author = value;
        self.store.save(&changed).map_err(|e| e.to_string())?;
        *settings = changed;
        Ok(())
    }

    /// Whether minimizing the window hides it in the tray.
    pub fn minimize_to_tray(&self) -> bool {
        lock(&self.settings).minimize_to_tray
    }

    /// Whether the user already answered the question about starting with Windows.
    pub fn startup_asked(&self) -> bool {
        lock(&self.settings).startup_asked
    }

    /// Saves where minimizing sends the window, and that the startup question was answered.
    pub fn save_window_choices(&self, minimize_to_tray: bool) -> Result<(), String> {
        let mut settings = lock(&self.settings);
        let mut changed = settings.clone();
        changed.minimize_to_tray = minimize_to_tray;
        changed.startup_asked = true;
        self.store.save(&changed).map_err(|e| e.to_string())?;
        *settings = changed;
        Ok(())
    }

    /// Returns the detail of the log file.
    pub fn log_level(&self) -> String {
        lock(&self.settings).log_level.clone()
    }

    /// Saves the detail of the log file: `error`, `warn`, `info` or `debug`.
    pub fn set_log_level(&self, level: &str) -> Result<(), String> {
        if !LOG_LEVELS.contains(&level) {
            return Err(format!("Not a log level: {level}"));
        }
        let mut settings = lock(&self.settings);
        let mut changed = settings.clone();
        changed.log_level = level.to_owned();
        self.store.save(&changed).map_err(|e| e.to_string())?;
        *settings = changed;
        Ok(())
    }

    /// Returns the publishing preferences.
    pub fn preferences(&self) -> Preferences {
        lock(&self.settings).preferences.clone()
    }

    /// Validates, saves and applies a complete set of publishing preferences,
    /// and returns it as stored, with the lists normalized.
    ///
    /// Raises:
    ///     String: A value is out of range or the file cannot be written.
    pub fn set_preferences(&self, preferences: Preferences) -> Result<Preferences, String> {
        let clean = preferences.validated().map_err(|e| e.to_string())?;
        {
            let mut settings = lock(&self.settings);
            let mut changed = settings.clone();
            changed.preferences = clean.clone();
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
        }
        self.service.set_preferences(clean.clone());
        self.apply_idle();
        Ok(clean)
    }

    /// Records how long the computer has gone without keyboard or mouse input.
    ///
    /// Args:
    ///     idle_for: Time since the last input; `None` when it cannot be read,
    ///         which never hides anything.
    pub fn update_idle(&self, idle_for: Option<Duration>) {
        *lock(&self.idle_for) = idle_for;
        self.apply_idle();
    }

    /// Hides everything while a program of the person's list runs or inside their
    /// hours, and shows it again after.
    ///
    /// Args:
    ///     minute: Minutes after midnight on the computer's clock; `None` when it
    ///         cannot be read, which leaves the hours out.
    pub fn update_quiet(&self, minute: Option<u16>) {
        let global = lock(&self.settings).preferences.global.clone();
        let running = if global.quiet_programs.is_empty() {
            Vec::new()
        } else {
            self.programs_within(QUIET_LIST_EVERY)
        };
        self.service
            .set_quiet(rpp_core::policy::is_quiet(&global, &running, minute));
    }

    /// The names of the running programs, from a list up to `max_age` old.
    fn programs_within(&self, max_age: Duration) -> Vec<String> {
        let mut cached = lock(&self.programs_seen);
        if let Some((at, names)) = &*cached
            && at.elapsed() < max_age
        {
            return names.clone();
        }
        let names: Vec<String> = match self.host.call(Request::ProcessList) {
            Ok(Value::Array(names)) => names
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            _ => Vec::new(),
        };
        *cached = Some((Instant::now(), names.clone()));
        names
    }

    /// Applies the idle rule to the last reading and the current setting.
    pub(crate) fn apply_idle(&self) {
        // Each lock is taken alone, so this can follow any other lock's release.
        let minutes = lock(&self.settings).preferences.global.idle_minutes;
        let idle_for = *lock(&self.idle_for);
        self.service
            .set_idle(rpp_core::policy::is_idle(idle_for, minutes));
    }

    /// Returns the browser extensions connected now and the last ones that were.
    pub fn extension_sessions(&self) -> ExtensionSessions {
        ExtensionSessions {
            live: self.bridge.live_sessions(),
            recent: self.bridge.recent_sessions(),
        }
    }

    /// Returns the code that pairs the browser extension.
    pub fn pairing(&self) -> Pairing {
        let port = self.bridge.address().port();
        let token = self.bridge.token().to_owned();
        Pairing {
            code: format!("{port}:{token}"),
            port,
            token,
        }
    }

    /// Stops publishing, removes the activity and closes the bridge.
    ///
    /// Waits for threads: call it from a blocking context.
    pub async fn shutdown(&self) {
        self.flush_usage();
        *lock(&self.local_api) = None;
        self.stop_watching();
        self.service.shutdown();
        self.bridge.shutdown().await;
    }
}
