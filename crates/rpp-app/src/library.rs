//! The presences found on disk, with their translations.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use rpp_catalog::Catalog;
use rpp_catalog::catalog::modified_message;
use rpp_config::{Found, Origin, Problem, Settings, load_presences_ranked};
use rpp_core::locale::{self, Table};
use rpp_plugins::{identity, resolve_settings};
use rpp_service::Presence;

/// Every presence the folders hold at the moment they were read.
#[derive(Debug, Default)]
pub struct Library {
    /// The presences, sorted by name.
    pub presences: Vec<Presence>,
    /// Folders that could not be loaded.
    pub problems: Vec<Problem>,
    locales: HashMap<String, BTreeMap<String, Table>>,
    folders: HashMap<String, PathBuf>,
    origins: HashMap<String, Origin>,
    /// Logos stored in the presence folders, ready for an `<img>` tag.
    icons: HashMap<String, String>,
}

/// Largest logo file read, in bytes.
const MAX_ICON: u64 = 256 * 1024;

/// A logo file as a `data:` address, so the window never sees a file path.
///
/// Returns:
///     The address, or `None` if the file is missing, too large, not an image
///     of the kinds allowed, or reached through a link out of the folder.
fn read_icon(folder: &Path, relative: &str) -> Option<String> {
    let path = fs::canonicalize(folder.join(relative)).ok()?;
    if !path.starts_with(fs::canonicalize(folder).ok()?)
        || fs::metadata(&path).ok()?.len() > MAX_ICON
    {
        return None;
    }
    let kind = match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => return None,
    };
    let bytes = fs::read(&path).ok()?;
    Some(format!("data:{kind};base64,{}", STANDARD.encode(bytes)))
}

/// The logo of a presence when it is a file in its folder.
fn local_icon(presence: &Presence, folder: Option<&Path>) -> Option<String> {
    let icon = presence.manifest.icon.as_deref()?;
    if icon.starts_with("https://") {
        return None;
    }
    read_icon(folder?, icon)
}

/// The options a presence declares, with the user's values where they are valid.
pub fn options_of(
    presence: &Presence,
    settings: &Settings,
) -> serde_json::Map<String, serde_json::Value> {
    let empty = BTreeMap::new();
    let saved = settings
        .options
        .get(&presence.manifest.name)
        .unwrap_or(&empty);
    resolve_settings(&presence.manifest.settings, saved)
}

/// The language in use: the saved choice, or the system's while it is `auto`.
pub fn effective_language(settings: &Settings, system: &str) -> String {
    if settings.language == "auto" {
        system.to_owned()
    } else {
        settings.language.clone()
    }
}

impl Library {
    /// Reads the presences in the given folders.
    ///
    /// Args:
    ///     roots: Folders with one subfolder per presence, in priority order.
    ///     catalog: Tells which folders of the first root it installed; a
    ///         catalog folder changed by hand is skipped.
    pub fn load(roots: &[PathBuf], catalog: Option<&Catalog>) -> Self {
        let user = roots.first();
        let classify = |folder: &Path| -> Result<Origin, String> {
            if folder.parent() != user.map(PathBuf::as_path) {
                return Ok(Origin::Bundled);
            }
            let Some(record) = catalog.and_then(|catalog| catalog.installed_at(folder)) else {
                return Ok(Origin::Local);
            };
            modified_message(&record).map_or(Ok(Origin::Catalog), Err)
        };
        let report = load_presences_ranked(roots, &|name| std::env::var(name).ok(), &classify);
        let icons = report
            .presences
            .iter()
            .filter_map(|presence| {
                let name = &presence.manifest.name;
                let icon = local_icon(presence, report.folders.get(name).map(PathBuf::as_path))?;
                Some((name.clone(), icon))
            })
            .collect();
        Self {
            presences: report.presences,
            problems: report.problems,
            locales: report.locales,
            folders: report.folders,
            origins: report.origins,
            icons,
        }
    }

    /// The logos kept as files in the presence folders, by presence name.
    pub fn local_icons(&self) -> &HashMap<String, String> {
        &self.icons
    }

    /// The folder a presence was read from.
    pub fn folder_of(&self, name: &str) -> Option<&Path> {
        self.folders.get(name).map(PathBuf::as_path)
    }

    /// Where a presence came from; only a local one may be modified.
    pub fn origin_of(&self, name: &str) -> Option<Origin> {
        self.origins.get(name).copied()
    }

    /// The name of the presence read from a folder.
    pub fn name_at(&self, folder: &Path) -> Option<&str> {
        self.folders
            .iter()
            .find(|(_, known)| known.as_path() == folder)
            .map(|(name, _)| name.as_str())
    }

    /// Replaces the presence read from a folder with a fresh reading of it.
    ///
    /// Raises:
    ///     String: Another folder already holds a presence with the new name,
    ///         ignoring case and separators.
    pub fn replace(&mut self, folder: &Path, found: Found) -> Result<(), String> {
        let name = found.presence.manifest.name.clone();
        let wanted = identity(&name);
        let taken = self
            .folders
            .iter()
            .any(|(other, known)| known != folder && identity(other) == wanted);
        if taken {
            return Err(format!("{name} is already the name of another presence."));
        }
        let mut origin = Origin::Local;
        if let Some(old) = self.name_at(folder).map(str::to_owned) {
            origin = self.origins.remove(&old).unwrap_or(origin);
            self.presences
                .retain(|presence| presence.manifest.name != old);
            self.locales.remove(&old);
            self.folders.remove(&old);
            self.icons.remove(&old);
        }
        self.origins.insert(name.clone(), origin);
        if let Some(icon) = local_icon(&found.presence, Some(folder)) {
            self.icons.insert(name.clone(), icon);
        }
        self.locales.insert(name.clone(), found.locales);
        self.folders.insert(name, folder.to_owned());
        self.presences.push(found.presence);
        self.presences
            .sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        Ok(())
    }

    /// Names present in both libraries whose manifest, script or texts differ.
    pub fn changed_since(&self, newer: &Library) -> Vec<String> {
        self.presences
            .iter()
            .filter_map(|old| {
                let name = &old.manifest.name;
                let current = newer.find(name)?;
                let same = old.manifest == current.manifest
                    && old.source == current.source
                    && self.locales.get(name) == newer.locales.get(name);
                (!same).then(|| name.clone())
            })
            .collect()
    }

    /// Finds a presence by name.
    pub fn find(&self, name: &str) -> Option<&Presence> {
        self.presences
            .iter()
            .find(|presence| presence.manifest.name == name)
    }

    /// A copy of a presence ready to run: texts in the language in use and
    /// the options the user picked, with defaults for the rest.
    pub fn prepared(&self, presence: &Presence, settings: &Settings, language: &str) -> Presence {
        let mut copy = presence.clone();
        copy.strings = self.strings(&presence.manifest.name, language);
        copy.settings = options_of(presence, settings);
        // The sites the person added join those of the manifest, so the extension
        // reads them and asks for each like for any other.
        if let Some(page) = copy.manifest.page.as_mut().filter(|page| page.user_sites) {
            for host in settings
                .sites
                .get(&presence.manifest.name)
                .into_iter()
                .flatten()
            {
                let pattern = rpp_plugins::PageSpec::user_pattern(host);
                if !page.matches.contains(&pattern) {
                    page.matches.push(pattern);
                }
            }
        }
        copy
    }

    /// The texts of a presence in a language, without copying the presence.
    pub fn strings(&self, name: &str, language: &str) -> Table {
        self.locales
            .get(name)
            .map(|tables| locale::resolve(tables, language))
            .unwrap_or_default()
    }

    /// The presences the user has not turned off, with their texts.
    pub fn enabled(&self, settings: &Settings, language: &str) -> Vec<Presence> {
        self.presences
            .iter()
            .filter(|presence| settings.enabled.contains(&presence.manifest.name))
            .map(|presence| self.prepared(presence, settings, language))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sites_a_person_added_join_the_rule_of_the_presence() {
        let manifest = rpp_plugins::Manifest::parse(
            r#"{"name":"Server","version":"1","client_id":"12345678901234567",
                "page":{"matches":["*://demo.example.com/*"],"user_sites":true,
                        "extract":{"t":{"from":"meta","name":"og:title"}}}}"#,
        )
        .unwrap();
        let presence = Presence {
            manifest,
            source: String::new(),
            strings: Table::new(),
            settings: serde_json::Map::new(),
        };
        let library = Library {
            presences: vec![presence],
            ..Library::default()
        };
        let mut settings = Settings::default();
        settings.enabled.insert("Server".into());
        let matches = |settings: &Settings| {
            let enabled = library.enabled(settings, "en");
            rpp_bridge::Rule::from_manifests(enabled.iter().map(|p| &p.manifest))[0]
                .matches
                .clone()
        };
        assert_eq!(matches(&settings), ["*://demo.example.com/*"]);
        settings.sites.insert(
            "Server".into(),
            vec!["media.home".into(), "demo.example.com".into()],
        );
        assert_eq!(
            matches(&settings),
            ["*://demo.example.com/*", "*://media.home/*"]
        );
        // The copy the library keeps is left as the manifest wrote it.
        assert_eq!(
            library.presences[0]
                .manifest
                .page
                .as_ref()
                .unwrap()
                .matches
                .len(),
            1
        );
    }
}
