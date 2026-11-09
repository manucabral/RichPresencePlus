//! Settings for a test that starts the application.

use std::fs;
use std::path::{Path, PathBuf};

/// Turns on every presence found in `roots`, as a user who chose them would
/// have, unless the test already wrote its own settings in `data_dir`.
///
/// Presences start off in the application; most tests are about what a
/// running presence does, so they begin from this.
pub fn turn_on_found(data_dir: &Path, roots: &[PathBuf]) {
    let store = rpp_config::SettingsStore::new(data_dir);
    if data_dir.join("settings.json").exists() {
        return;
    }
    let mut settings = rpp_config::Settings::default();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(text) = fs::read_to_string(entry.path().join("manifest.json")) else {
                continue;
            };
            let name = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|manifest| manifest["name"].as_str().map(str::to_owned));
            if let Some(name) = name {
                settings.enabled.insert(name);
            }
        }
    }
    store
        .save(&settings)
        .expect("the test can write its settings");
}
