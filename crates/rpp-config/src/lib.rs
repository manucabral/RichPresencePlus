//! What the application keeps between runs: user settings and the presences
//! found in its folders.

pub mod loader;
pub mod manual;
pub mod profiles;
pub mod settings;
pub mod shortcut;

pub use loader::{
    Found, LoadReport, Origin, Problem, load_presence, load_presences, load_presences_checked,
    load_presences_ranked, load_presences_with,
};
pub use manual::{
    MAX_PRESETS, MAX_ROTATION_SECONDS, MAX_SHARED, MIN_ROTATION_SECONDS, Manual, ManualPreset,
    Rotation,
};
pub use profiles::{MAX_PROFILES, Profile};
pub use settings::{
    DEFAULT_LOG_LEVEL, DEFAULT_PORT, LOG_LEVELS, Loaded, MAX_FAVORITES, Settings, SettingsStore,
};
