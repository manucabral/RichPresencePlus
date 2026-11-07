//! Switching between named sets of publishing rules.

use rpp_config::Profile;
use rpp_config::profiles;
use rpp_plugins::Host;

use crate::controller::{Controller, lock};

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// Returns the saved profiles.
    pub fn profiles(&self) -> Vec<Profile> {
        lock(&self.settings).profiles.clone()
    }

    /// Saves the current rules and the presences that are off under a name.
    pub fn save_profile(&self, name: &str) -> Result<(), String> {
        let mut settings = lock(&self.settings);
        let profile = Profile {
            name: name.to_owned(),
            preferences: settings.preferences.clone(),
            disabled: settings.disabled.clone(),
        }
        .validated()?;
        let mut changed = settings.clone();
        profiles::save(&mut changed.profiles, profile)?;
        self.store.save(&changed).map_err(|e| e.to_string())?;
        *settings = changed;
        Ok(())
    }

    /// Deletes a profile.
    pub fn delete_profile(&self, name: &str) -> Result<(), String> {
        let mut settings = lock(&self.settings);
        let mut changed = settings.clone();
        if !profiles::delete(&mut changed.profiles, name) {
            return Err(format!("There is no profile called {name}."));
        }
        self.store.save(&changed).map_err(|e| e.to_string())?;
        *settings = changed;
        Ok(())
    }

    /// Makes a profile the current rules and turns presences on or off to match.
    pub fn apply_profile(&self, name: &str) -> Result<(), String> {
        let preferences = {
            let mut settings = lock(&self.settings);
            let profile = settings
                .profiles
                .iter()
                .find(|profile| profile.name == name)
                .cloned()
                .ok_or_else(|| format!("There is no profile called {name}."))?;
            // The file can be edited by hand; check again before using it.
            let preferences = profile.preferences.validated().map_err(|e| e.to_string())?;
            let mut changed = settings.clone();
            changed.preferences = preferences.clone();
            changed.disabled = profile.disabled;
            self.store.save(&changed).map_err(|e| e.to_string())?;
            *settings = changed;
            preferences
        };
        self.service.set_preferences(preferences);
        self.apply_idle();
        self.sync_running();
        tracing::info!(profile = %name, "publishing profile applied");
        Ok(())
    }
}
