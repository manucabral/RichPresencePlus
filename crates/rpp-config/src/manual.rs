//! Activities the user writes by hand, kept as named presets.

use rpp_core::Activity;
use serde::{Deserialize, Serialize};

/// Most presets kept.
pub const MAX_PRESETS: usize = 50;
/// Longest preset name.
const MAX_NAME: usize = 64;
/// Largest shared custom status read, in bytes.
pub const MAX_SHARED: usize = 16 * 1024;
/// Names the text a custom status is shared as.
const SHARED_FORMAT: &str = "rich-presence-plus/status";
/// Layout of the shared text this build writes and reads.
const SHARED_VERSION: u32 = 1;

/// A custom status as text, to send to someone or keep in a file.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SharedStatus {
    format: String,
    version: u32,
    status: SharedPreset,
}

/// The preset inside a shared text; unknown fields are refused.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SharedPreset {
    name: String,
    client_id: String,
    activity: Activity,
    #[serde(default)]
    show_elapsed: bool,
}

/// One activity written by the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualPreset {
    /// Name shown in the list; also its identifier.
    pub name: String,
    /// Discord application that shows the activity; its name is the title on Discord.
    pub client_id: String,
    /// What Discord shows. `start` and `end` are ignored; see `show_elapsed`.
    pub activity: Activity,
    /// Count the time since the preset was turned on.
    #[serde(default)]
    pub show_elapsed: bool,
}

/// Shortest time each status stays in a rotation. Discord accepts five
/// updates per 20 seconds; 15 leaves room for the presences.
pub const MIN_ROTATION_SECONDS: u32 = 15;
/// Longest time each status stays in a rotation.
pub const MAX_ROTATION_SECONDS: u32 = 3600;

/// Saved presets shown one after another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rotation {
    /// Preset names, in the order they are shown.
    pub names: Vec<String>,
    /// How long each one stays, in seconds.
    pub seconds: u32,
}

impl Rotation {
    /// Checks the rotation against the saved presets.
    ///
    /// Raises:
    ///     String: It has fewer than two presets, repeats or misses one,
    ///         or its time is outside 15 seconds to an hour.
    pub fn validated(self, presets: &[ManualPreset]) -> Result<Self, String> {
        if !(MIN_ROTATION_SECONDS..=MAX_ROTATION_SECONDS).contains(&self.seconds) {
            return Err(format!(
                "Each status stays {MIN_ROTATION_SECONDS} to {MAX_ROTATION_SECONDS} seconds."
            ));
        }
        let unique: std::collections::HashSet<&String> = self.names.iter().collect();
        if self.names.len() < 2 || unique.len() != self.names.len() {
            return Err("Choose two or more different statuses.".into());
        }
        if let Some(missing) = self
            .names
            .iter()
            .find(|name| !presets.iter().any(|preset| &preset.name == *name))
        {
            return Err(format!("There is no custom status called {missing}."));
        }
        Ok(self)
    }

    /// The preset on display after `elapsed` seconds of rotation.
    pub fn at(&self, elapsed: u64) -> &str {
        let step = elapsed / u64::from(self.seconds.max(1));
        let index = (step % self.names.len().max(1) as u64) as usize;
        self.names.get(index).map_or("", String::as_str)
    }
}

/// The presets and which one is on display.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Manual {
    /// Saved presets, in the order they were created.
    pub presets: Vec<ManualPreset>,
    /// Name of the preset shown now, if any.
    pub active: Option<String>,
    /// Presets shown one after another; a manual choice ends it.
    pub rotation: Option<Rotation>,
}

impl ManualPreset {
    /// Trims the name, validates the activity and returns the preset ready to
    /// store.
    ///
    /// Raises:
    ///     String: Why the preset cannot be used.
    pub fn validated(mut self) -> Result<Self, String> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > MAX_NAME {
            return Err(format!("The name needs 1 to {MAX_NAME} characters."));
        }
        let id_ok = (17..=20).contains(&self.client_id.len())
            && self.client_id.bytes().all(|b| b.is_ascii_digit());
        if !id_ok {
            return Err(
                "The application id is the number Discord shows for the application.".into(),
            );
        }
        self.activity.start = None;
        self.activity.end = None;
        self.activity = self.activity.validated().map_err(|e| e.to_string())?;
        if self.activity.is_blank() {
            return Err("Write at least one line of text or an image.".into());
        }
        Ok(self)
    }
}

impl ManualPreset {
    /// The preset as indented JSON, with a format name and version, that
    /// another user can import.
    pub fn to_shared(&self) -> String {
        let shared = SharedStatus {
            format: SHARED_FORMAT.into(),
            version: SHARED_VERSION,
            status: SharedPreset {
                name: self.name.clone(),
                client_id: self.client_id.clone(),
                activity: self.activity.clone(),
                show_elapsed: self.show_elapsed,
            },
        };
        // Serializing plain owned data cannot fail.
        serde_json::to_string_pretty(&shared).unwrap_or_default()
    }

    /// Reads a shared custom status and checks it like one typed by hand.
    ///
    /// Args:
    ///     text: What [`ManualPreset::to_shared`] wrote, possibly edited.
    ///
    /// Raises:
    ///     String: If the text is too long, not a shared status, of a newer
    ///         layout, or holds a preset that would not be accepted.
    pub fn from_shared(text: &str) -> Result<Self, String> {
        if text.len() > MAX_SHARED {
            return Err("That text is too long to be a custom status.".into());
        }
        let shared: SharedStatus = serde_json::from_str(text.trim())
            .map_err(|_| "That text is not a custom status from Rich Presence Plus.".to_owned())?;
        if shared.format != SHARED_FORMAT {
            return Err("That text is not a custom status from Rich Presence Plus.".into());
        }
        if shared.version != SHARED_VERSION {
            return Err(
                "That custom status was written by a newer version; update the app.".into(),
            );
        }
        ManualPreset {
            name: shared.status.name,
            client_id: shared.status.client_id,
            activity: shared.status.activity,
            show_elapsed: shared.status.show_elapsed,
        }
        .validated()
    }
}

impl Manual {
    /// Adds an already validated preset or replaces the one with its name.
    ///
    /// Raises:
    ///     String: It is new and the list is full.
    pub fn save(&mut self, preset: ManualPreset) -> Result<(), String> {
        if let Some(existing) = self.presets.iter_mut().find(|p| p.name == preset.name) {
            *existing = preset;
            return Ok(());
        }
        if self.presets.len() >= MAX_PRESETS {
            return Err(format!("Keep at most {MAX_PRESETS} presets."));
        }
        self.presets.push(preset);
        Ok(())
    }

    /// Removes a preset, and stops showing it if it was on. True when a preset
    /// had that name.
    pub fn delete(&mut self, name: &str) -> bool {
        let before = self.presets.len();
        self.presets.retain(|p| p.name != name);
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }
        // A rotation keeps going without it, if two statuses are left.
        if let Some(rotation) = &mut self.rotation {
            rotation.names.retain(|n| n != name);
            if rotation.names.len() < 2 {
                self.rotation = None;
            }
        }
        self.presets.len() != before
    }

    /// The preset on display, if it still exists.
    pub fn shown(&self) -> Option<&ManualPreset> {
        let name = self.active.as_deref()?;
        self.presets.iter().find(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(name: &str, details: &str) -> ManualPreset {
        ManualPreset {
            name: name.into(),
            client_id: "12345678901234567".into(),
            activity: Activity {
                details: Some(details.into()),
                start: Some(5),
                ..Activity::default()
            },
            show_elapsed: true,
        }
    }

    fn rotation(names: &[&str], seconds: u32) -> Rotation {
        Rotation {
            names: names.iter().map(|n| (*n).to_owned()).collect(),
            seconds,
        }
    }

    #[test]
    fn a_rotation_needs_two_saved_statuses_and_a_sane_time() {
        let presets = [preset("A", "a"), preset("B", "b"), preset("C", "c")];
        assert!(rotation(&["A", "B"], 15).validated(&presets).is_ok());
        for bad in [
            rotation(&["A"], 30),
            rotation(&["A", "A"], 30),
            rotation(&["A", "Missing"], 30),
            rotation(&["A", "B"], 14),
            rotation(&["A", "B"], 3601),
        ] {
            assert!(bad.clone().validated(&presets).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_rotation_steps_through_its_statuses_in_order() {
        let r = rotation(&["A", "B", "C"], 20);
        let shown: Vec<&str> = [0, 19, 20, 40, 59, 60, 1000]
            .iter()
            .map(|s| r.at(*s))
            .collect();
        // 1000 s is step 50, and 50 % 3 is 2.
        assert_eq!(shown, ["A", "A", "B", "C", "C", "A", "C"]);
    }

    #[test]
    fn deleting_a_status_takes_it_out_of_the_rotation() {
        let mut manual = Manual {
            presets: vec![preset("A", "a"), preset("B", "b"), preset("C", "c")],
            active: Some("A".into()),
            rotation: Some(rotation(&["A", "B", "C"], 20)),
        };
        manual.delete("B");
        assert_eq!(manual.rotation.as_ref().unwrap().names, ["A", "C"]);
        manual.delete("C");
        assert_eq!(manual.rotation, None);
    }

    #[test]
    fn a_shared_status_round_trips_and_is_checked_on_the_way_in() {
        let original = preset("Studying", "Reading");
        let text = original.validated().unwrap().to_shared();
        assert!(text.contains("rich-presence-plus/status"));
        let back = ManualPreset::from_shared(&text).unwrap();
        assert_eq!(back.name, "Studying");
        assert_eq!(back.activity.details.as_deref(), Some("Reading"));
        // Times never travel: they belong to whoever turns it on.
        assert_eq!(back.activity.start, None);

        let tampered = text.replace("12345678901234567", "not-a-number");
        assert!(ManualPreset::from_shared(&tampered).is_err());
        let unknown = text.replacen("\"version\": 1,", "\"version\": 1, \"run\": \"x\",", 1);
        assert!(ManualPreset::from_shared(&unknown).is_err());
        let newer = text.replace("\"version\": 1", "\"version\": 2");
        assert!(
            ManualPreset::from_shared(&newer)
                .unwrap_err()
                .contains("newer")
        );
        let link = text.replace(
            "\"buttons\": []",
            "\"buttons\": [{\"label\": \"Go\", \"url\": \"javascript:alert(1)\"}]",
        );
        assert_ne!(link, text);
        assert!(ManualPreset::from_shared(&link).is_err());
        assert!(ManualPreset::from_shared("{}").is_err());
        assert!(ManualPreset::from_shared(&" ".repeat(MAX_SHARED + 1)).is_err());
    }

    #[test]
    fn a_valid_preset_is_cleaned() {
        let clean = preset("  Studying  ", "Reading").validated().unwrap();
        assert_eq!(clean.name, "Studying");
        assert_eq!(clean.activity.start, None);
    }

    #[test]
    fn bad_presets_are_refused_with_a_reason() {
        let mut no_id = preset("A", "x");
        no_id.client_id = "abc".into();
        let mut blank = preset("A", "x");
        blank.activity.details = None;
        let mut long = preset("A", "x");
        long.activity.state = Some("s".repeat(200));
        for bad in [preset(" ", "x"), no_id, blank, long] {
            assert!(bad.validated().is_err());
        }
    }

    #[test]
    fn saving_replaces_by_name_and_deleting_turns_it_off() {
        let mut manual = Manual::default();
        manual.save(preset("A", "one")).unwrap();
        manual.save(preset("A", "two")).unwrap();
        manual.save(preset("B", "three")).unwrap();
        assert_eq!(manual.presets.len(), 2);
        assert_eq!(manual.presets[0].activity.details.as_deref(), Some("two"));
        manual.active = Some("A".into());
        assert!(manual.shown().is_some());
        assert!(manual.delete("A"));
        assert_eq!(manual.active, None);
        assert!(!manual.delete("A"));
    }

    #[test]
    fn the_list_has_a_limit() {
        let mut manual = Manual::default();
        for n in 0..MAX_PRESETS {
            manual.save(preset(&format!("P{n}"), "x")).unwrap();
        }
        assert!(manual.save(preset("one more", "x")).is_err());
        // Replacing an existing one still works when full.
        assert!(manual.save(preset("P0", "y")).is_ok());
    }
}
