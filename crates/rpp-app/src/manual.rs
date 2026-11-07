//! Activities the user writes by hand, shown through the same selection as scripts.

use std::time::{SystemTime, UNIX_EPOCH};

use rpp_config::{ManualPreset, Rotation};
use rpp_core::{Candidate, Kind};
use rpp_plugins::Host;

use crate::controller::{Controller, lock};
use crate::views::ManualView;

/// Source name manual activities are offered under.
pub const SOURCE: &str = ".manual";

/// The candidate for a preset.
///
/// Args:
///     since: Unix second the preset was turned on, used when it counts time.
pub fn candidate(preset: &ManualPreset, since: u64) -> Candidate {
    let mut activity = preset.activity.clone();
    activity.start = preset.show_elapsed.then_some(since);
    Candidate {
        source: SOURCE.to_owned(),
        label: preset.name.clone(),
        client_id: preset.client_id.clone(),
        activity,
        kind: Kind::Manual,
        foreground: false,
        paused: false,
        hide_when_paused: true,
        domains: Vec::new(),
        application: String::new(),
        // A manual activity stays until the user turns it off.
        expires_at: None,
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// Shows again the preset that was on when the application closed.
    pub(crate) fn offer_saved_manual(&self) {
        let shown = lock(&self.settings).manual.shown().cloned();
        if let Some(preset) = shown {
            let since = now();
            *lock(&self.manual_since) = since;
            // A saved rotation starts over from its first status.
            *lock(&self.rotation_since) = since;
            self.service.offer(candidate(&preset, since));
        }
    }

    /// Returns the saved presets and the one on display.
    pub fn manual(&self) -> ManualView {
        let settings = lock(&self.settings);
        ManualView {
            presets: settings.manual.presets.clone(),
            active: settings.manual.active.clone(),
            rotation: settings.manual.rotation.clone(),
        }
    }

    /// Saves a preset, replacing the one with the same name.
    ///
    /// Raises:
    ///     String: The preset is not valid, the list is full or it cannot be saved.
    pub fn save_manual_preset(&self, preset: ManualPreset) -> Result<(), String> {
        let clean = preset.validated()?;
        let on_display = {
            let mut settings = lock(&self.settings);
            let mut changed = settings.clone();
            changed.manual.save(clean.clone())?;
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
            settings.manual.active.as_deref() == Some(clean.name.as_str())
        };
        if on_display {
            let since = *lock(&self.manual_since);
            self.service.offer(candidate(&clean, since));
        }
        Ok(())
    }

    /// Writes a saved preset as text to share.
    pub fn export_manual_preset(&self, name: &str) -> Result<String, String> {
        lock(&self.settings)
            .manual
            .presets
            .iter()
            .find(|preset| preset.name == name)
            .map(ManualPreset::to_shared)
            .ok_or_else(|| format!("There is no custom status called {name}."))
    }

    /// Reads a shared preset for the user to review; nothing is saved.
    pub fn import_manual_preset(&self, text: &str) -> Result<ManualPreset, String> {
        ManualPreset::from_shared(text)
    }

    /// Deletes a preset; it stops being shown if it was on.
    pub fn delete_manual_preset(&self, name: &str) -> Result<(), String> {
        let was_on_display = {
            let mut settings = lock(&self.settings);
            let on_display = settings.manual.active.as_deref() == Some(name);
            let mut changed = settings.clone();
            if !changed.manual.delete(name) {
                return Err(format!("There is no preset called {name}."));
            }
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
            on_display
        };
        if was_on_display {
            self.service.withdraw(SOURCE);
        }
        Ok(())
    }

    /// Shows saved statuses one after another, starting with the first.
    ///
    /// Args:
    ///     names: Two or more saved statuses, in the order to show them.
    ///     seconds: How long each one stays, 15 to 3600.
    pub fn start_rotation(&self, names: Vec<String>, seconds: u32) -> Result<(), String> {
        let first = {
            let mut settings = lock(&self.settings);
            let rotation = Rotation { names, seconds }.validated(&settings.manual.presets)?;
            let first = rotation.names[0].clone();
            let preset = settings
                .manual
                .presets
                .iter()
                .find(|preset| preset.name == first)
                .cloned()
                .ok_or_else(|| format!("There is no custom status called {first}."))?;
            let mut changed = settings.clone();
            changed.manual.active = Some(first);
            changed.manual.rotation = Some(rotation);
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
            preset
        };
        let since = now();
        *lock(&self.manual_since) = since;
        *lock(&self.rotation_since) = since;
        self.service.offer(candidate(&first, since));
        Ok(())
    }

    /// Ends the rotation; the status on display stays.
    pub fn stop_rotation(&self) -> Result<(), String> {
        let mut settings = lock(&self.settings);
        let mut changed = settings.clone();
        changed.manual.rotation = None;
        self.store.save(&changed).map_err(|e| e.to_string())?;
        *settings = changed;
        Ok(())
    }

    /// Moves a rotation to the status due at `now`; does nothing otherwise.
    pub fn tick_rotation(&self, now: u64) {
        let next = {
            let mut settings = lock(&self.settings);
            let Some(rotation) = &settings.manual.rotation else {
                return;
            };
            let since = *lock(&self.rotation_since);
            let due = rotation.at(now.saturating_sub(since)).to_owned();
            if settings.manual.active.as_deref() == Some(due.as_str()) {
                return;
            }
            let Some(preset) = settings
                .manual
                .presets
                .iter()
                .find(|p| p.name == due)
                .cloned()
            else {
                return;
            };
            settings.manual.active = Some(due);
            preset
        };
        let since = *lock(&self.manual_since);
        self.service.offer(candidate(&next, since));
    }

    /// Shows a preset, or stops showing the current one when `name` is `None`.
    pub fn show_manual(&self, name: Option<&str>) -> Result<(), String> {
        let shown = {
            let mut settings = lock(&self.settings);
            let preset = match name {
                Some(name) => Some(
                    settings
                        .manual
                        .presets
                        .iter()
                        .find(|preset| preset.name == name)
                        .cloned()
                        .ok_or_else(|| format!("There is no preset called {name}."))?,
                ),
                None => None,
            };
            let mut changed = settings.clone();
            changed.manual.active = preset.as_ref().map(|preset| preset.name.clone());
            // Choosing a status by hand ends a rotation.
            changed.manual.rotation = None;
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
            preset
        };
        match shown {
            Some(preset) => {
                let since = now();
                *lock(&self.manual_since) = since;
                self.service.offer(candidate(&preset, since));
            }
            None => self.service.withdraw(SOURCE),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rpp_core::Activity;

    fn preset(show_elapsed: bool) -> ManualPreset {
        ManualPreset {
            name: "Studying".into(),
            client_id: "12345678901234567".into(),
            activity: Activity {
                details: Some("Reading".into()),
                ..Activity::default()
            },
            show_elapsed,
        }
    }

    #[test]
    fn the_candidate_ranks_as_manual_and_never_expires() {
        let shown = candidate(&preset(false), 100);
        assert_eq!(shown.source, SOURCE);
        assert_eq!(shown.label, "Studying");
        assert_eq!(shown.kind, Kind::Manual);
        assert_eq!(shown.expires_at, None);
        assert_eq!(shown.activity.start, None);
    }

    #[test]
    fn the_clock_starts_when_the_preset_was_turned_on() {
        assert_eq!(candidate(&preset(true), 1234).activity.start, Some(1234));
    }
}
