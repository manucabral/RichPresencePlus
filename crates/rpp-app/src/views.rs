//! Values the interface receives, in the field names it reads.

use rpp_config::{ManualPreset, Origin, Problem};
use rpp_core::{Activity, Kind};
pub use rpp_plugins::Grant;
use rpp_plugins::Meta;
use rpp_plugins::Setting;
use rpp_service::Status;
use serde::Serialize;
use serde_json::Value;

/// What a presence may do, as the user should see it before turning it on.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Access {
    /// Hostnames it may contact over HTTPS.
    pub http: Vec<String>,
    /// It can list running programs.
    pub process: bool,
    /// Options it reads from a program's command line, by executable name.
    pub args: std::collections::BTreeMap<String, Vec<String>>,
    /// It can read the title of the active window.
    pub window: bool,
    /// It can read the system media session.
    pub media: bool,
    /// Game server protocols it uses to ask servers for their public information.
    pub query: Vec<String>,
    /// Folders whose files it can read.
    pub files: Vec<String>,
    /// Sites it reads through the browser extension.
    pub sites: Vec<String>,
}

impl Access {
    /// Sorts what a presence reaches, or what an update adds, into the fields
    /// the interface reads.
    pub fn of(grants: &[Grant]) -> Self {
        let mut access = Self::default();
        for grant in grants.iter().cloned() {
            match grant {
                Grant::Http(host) => access.http.push(host),
                Grant::Process => access.process = true,
                Grant::Arg { program, option } => {
                    access.args.entry(program).or_default().push(option);
                }
                Grant::Window => access.window = true,
                Grant::Media => access.media = true,
                Grant::File(folder) => access.files.push(folder),
                Grant::Query(protocol) => access.query.push(protocol),
                Grant::Site(pattern) => access.sites.push(pattern),
            }
        }
        access
    }
}

/// An option a presence offers, with its current value and its texts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OptionView {
    /// The declaration: kind, key, default and limits.
    #[serde(flatten)]
    pub setting: Setting,
    /// Text shown next to the control, in the language in use.
    pub label: String,
    /// The value the script receives now.
    pub value: Value,
    /// Text for each choice of a `choice` option, in the same order.
    pub choice_labels: Vec<String>,
    /// One line that says what the option does, from `help.<key>` of the presence's texts.
    pub help: Option<String>,
}

/// A presence in the list.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PresenceView {
    /// Name, also its identifier.
    pub name: String,
    /// Version text.
    pub version: String,
    /// Short explanation.
    pub description: Option<String>,
    /// Group that sets its default priority.
    pub kind: Option<Kind>,
    /// Seconds between reads.
    pub interval: u32,
    /// The developer's choice of hiding it while paused; the user can change it.
    pub hide_when_paused: bool,
    /// The user has it turned on.
    pub enabled: bool,
    /// Its script is running now.
    pub running: bool,
    /// What it may access.
    pub access: Access,
    /// The options it offers.
    pub options: Vec<OptionView>,
    /// Where it came from.
    pub origin: Origin,
    /// The user wrote it, so it can be edited and watched.
    pub editable: bool,
    /// Who wrote it.
    pub author: Option<String>,
    /// Color of its card, as `#RRGGBB`.
    pub color: Option<String>,
    /// Words that describe it.
    pub tags: Vec<String>,
    /// Its logo when it is an `https://` address; a logo kept as a file comes
    /// from [`Controller::icons`](crate::Controller::icons) instead.
    pub icon: Option<String>,
}

/// The author written into new presences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorView {
    /// What the user saved in Settings.
    pub saved: Option<String>,
    /// The Discord user name, used while nothing is saved.
    pub discord: Option<String>,
}

/// How the extension is paired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pairing {
    /// Port the bridge listens on.
    pub port: u16,
    /// Secret the extension presents.
    pub token: String,
    /// `port:token`, the text the extension asks for.
    pub code: String,
}

/// Browser extensions connected now and the last ones that were.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtensionSessions {
    /// Open connections, oldest first.
    pub live: Vec<rpp_bridge::SessionView>,
    /// Ended connections, newest first; forgotten when the application closes.
    pub recent: Vec<rpp_bridge::SessionView>,
}

/// The language choice and what `auto` resolves to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageView {
    /// Saved choice: `auto` or a language tag.
    pub setting: String,
    /// Language of the system.
    pub system: String,
}

/// Everything the dashboard shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Overview {
    /// Application version.
    pub version: String,
    /// A Discord client is running.
    pub discord_running: bool,
    /// Extensions connected to the bridge.
    pub extensions: usize,
    /// Browser of each connected extension, sorted.
    pub browsers: Vec<rpp_bridge::Browser>,
    /// Discord account that receives the activity, once a handshake told it.
    pub discord_user: Option<String>,
    /// Publishing is paused by the user or by a damaged settings file.
    pub paused: bool,
    /// Problem to show at the top, if any.
    pub warning: Option<String>,
    /// What is shown on Discord and why.
    pub status: Status,
    /// Presence folders that could not be loaded.
    pub problems: Vec<Problem>,
}

/// What the tray icon paints. It is a small part of [`Overview`], read without
/// the rest of its cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glance {
    /// Language tag of the texts.
    pub language: String,
    /// Publishing is paused.
    pub paused: bool,
    /// A Discord client is running, as of a few seconds ago.
    pub discord_running: bool,
    /// Everything is hidden because the computer is idle.
    pub idle: bool,
    /// Everything is hidden because a rule of the person holds.
    pub quiet: bool,
    /// Name of the activity on display.
    pub showing: Option<String>,
}

/// Where a catalog presence stands on this computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogState {
    /// Not installed.
    Available,
    /// Installed and current.
    Installed,
    /// Installed, with a newer version in the catalog.
    Update,
    /// Installed, but a file was changed afterwards; it is not loaded until reinstalled.
    Modified,
}

/// One approved presence in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogItem {
    /// Name, also its identifier.
    pub name: String,
    /// Version in the catalog.
    pub version: String,
    /// What it shows, in English.
    pub description: Option<String>,
    /// Group: `game`, `video`, `music` and so on.
    pub kind: Option<String>,
    /// Download size of all its files, in bytes.
    pub size: u64,
    /// Whether it is installed here and whether it can be updated.
    pub state: CatalogState,
    /// Version installed here, when there is one.
    pub installed_version: Option<String>,
    /// Earlier versions that can be restored, newest first.
    pub history: Vec<String>,
    /// Who wrote it.
    pub author: Option<String>,
    /// Color of its card, as `#RRGGBB`.
    pub color: Option<String>,
    /// Words that describe it.
    pub tags: Vec<String>,
    /// Its logo, as an `https://` address.
    pub icon: Option<String>,
    /// The catalog version needs a newer application, so it cannot be installed.
    pub too_new: bool,
}

impl From<rpp_catalog::Listing> for CatalogItem {
    fn from(listing: rpp_catalog::Listing) -> Self {
        use rpp_catalog::State;
        let (state, installed_version) = match listing.state {
            State::Available => (CatalogState::Available, None),
            State::Installed { version } => (CatalogState::Installed, Some(version.to_string())),
            State::Update { installed } => (CatalogState::Update, Some(installed.to_string())),
            State::Modified { installed } => (CatalogState::Modified, Some(installed.to_string())),
        };
        let entry = listing.entry;
        Self {
            too_new: entry.too_new(),
            size: entry.files.values().map(|file| file.size).sum(),
            name: entry.name,
            version: entry.version.to_string(),
            description: entry.description,
            kind: entry.kind,
            state,
            installed_version,
            history: listing.history.iter().map(ToString::to_string).collect(),
            author: entry.author,
            color: entry.color,
            tags: entry.tags,
            icon: entry.icon,
        }
    }
}

/// The local address for overlays: whether it is on, and its port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LocalApiView {
    /// The person turned it on.
    pub on: bool,
    /// The port it listens on.
    pub port: u16,
}

/// The "Your week" page: whether time is counted and what was counted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsageView {
    /// The person turned the count on.
    pub on: bool,
    /// Seconds on display, by day (`YYYY-MM-DD`) and by presence.
    pub days: std::collections::BTreeMap<String, std::collections::BTreeMap<String, u64>>,
}

/// What came of asking the catalog for a presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum InstallOutcome {
    /// It is on disk and running.
    Installed,
    /// The update reaches for more. Nothing changed, and the installed copy keeps
    /// running with what it had.
    Asks {
        /// What it adds, to show to the person.
        access: Access,
        /// The same list, to send back with a yes.
        grants: Vec<Grant>,
    },
}

/// The catalog page: whether it can be used and what it lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogView {
    /// False when this build carries no signing key, so nothing can be installed.
    pub available: bool,
    /// The approved presences.
    pub items: Vec<CatalogItem>,
}

/// The manual presets and the one on display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManualView {
    /// Saved presets, in the order they were created.
    pub presets: Vec<ManualPreset>,
    /// Name of the preset shown now, if any.
    pub active: Option<String>,
    /// Statuses shown one after another, if a rotation is on.
    pub rotation: Option<rpp_config::Rotation>,
}

/// What running a presence once showed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Inspection {
    /// What `read` returned.
    pub snapshot: Option<Value>,
    /// What the presence would show on Discord.
    pub activity: Option<Activity>,
    /// What it says about its activity.
    pub meta: Meta,
    /// Why the script stopped, when it did.
    pub error: Option<String>,
    /// What the script printed with `console`.
    pub console: Vec<String>,
}

impl Inspection {
    /// An inspection that ended in an error.
    pub fn failed(error: String, console: Vec<String>) -> Self {
        Self {
            error: Some(error),
            console,
            ..Self::default()
        }
    }
}

/// A saved version of a presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionView {
    /// Its number; numbers never repeat.
    pub id: u32,
    /// When it was saved, in Unix seconds.
    pub at: u64,
    /// The version its manifest declared.
    pub version: String,
    /// What the author wrote about it.
    pub notes: String,
    /// How many files it holds.
    pub files: usize,
    /// Their total size in bytes.
    pub size: u64,
}

/// How a file differs from a saved version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Change {
    /// It exists now and did not then.
    Added,
    /// It existed then and does not now.
    Removed,
    /// Its contents differ.
    Changed,
}

/// One file that differs from a saved version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileChange {
    /// Its path inside the presence.
    pub path: String,
    /// How it differs.
    pub change: Change,
    /// Lines that are new; 0 for a file that is not text.
    pub added: usize,
    /// Lines that are gone; 0 for a file that is not text.
    pub removed: usize,
}

/// The outcome of replaying a test case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    /// The case.
    pub name: String,
    /// Whether the presence showed what was expected.
    pub passed: bool,
    /// What it should show.
    pub expected: Value,
    /// What it showed.
    pub actual: Value,
    /// Why the script stopped, when it did.
    pub error: Option<String>,
}

/// One rule a presence must follow to enter the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewCheck {
    /// Which rule, for the interface to name it.
    pub rule: &'static str,
    /// Whether the presence follows it.
    pub passed: bool,
    /// What to fix, when it does not.
    pub detail: Option<String>,
}

/// A clean copy of a presence, ready to upload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreparedView {
    /// Where the folder is.
    pub path: String,
    /// How many files it holds.
    pub files: usize,
    /// Their size in bytes.
    pub size: u64,
}

/// What the developer tools are doing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DevStatus {
    /// Name of the presence being watched.
    pub watching: Option<String>,
    /// Its folder.
    pub folder: Option<String>,
    /// Unix second of the last reload that worked.
    pub last_reload: Option<u64>,
    /// Why the last edit did not load, if it did not.
    pub error: Option<String>,
}
