//! Named sets of publishing rules the user can switch between.

use std::collections::BTreeSet;

use rpp_core::Preferences;
use serde::{Deserialize, Serialize};

/// Most profiles kept.
pub const MAX_PROFILES: usize = 20;
const MAX_NAME: usize = 64;

/// The publishing rules and the presences that were off when it was saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Name shown in the list; also its identifier.
    pub name: String,
    /// Privacy, priorities and per-presence choices.
    pub preferences: Preferences,
    /// Presences that are turned off.
    pub disabled: BTreeSet<String>,
}

impl Profile {
    /// Checks the profile and returns it ready to store.
    ///
    /// Raises:
    ///     String: The name or the preferences cannot be used.
    pub fn validated(mut self) -> Result<Self, String> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > MAX_NAME {
            return Err(format!("The name needs 1 to {MAX_NAME} characters."));
        }
        self.preferences = self.preferences.validated().map_err(|e| e.to_string())?;
        Ok(self)
    }
}

/// Adds a profile or replaces the one with the same name.
///
/// Raises:
///     String: It is new and the list is full.
pub fn save(list: &mut Vec<Profile>, profile: Profile) -> Result<(), String> {
    if let Some(existing) = list.iter_mut().find(|p| p.name == profile.name) {
        *existing = profile;
        return Ok(());
    }
    if list.len() >= MAX_PROFILES {
        return Err(format!("Keep at most {MAX_PROFILES} profiles."));
    }
    list.push(profile);
    Ok(())
}

/// Removes a profile; true when one had that name.
pub fn delete(list: &mut Vec<Profile>, name: &str) -> bool {
    let before = list.len();
    list.retain(|p| p.name != name);
    list.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> Profile {
        Profile {
            name: name.into(),
            preferences: Preferences::default(),
            disabled: BTreeSet::new(),
        }
    }

    #[test]
    fn names_are_trimmed_and_limited() {
        assert_eq!(profile("  Gaming ").validated().unwrap().name, "Gaming");
        assert!(profile("   ").validated().is_err());
        assert!(profile(&"n".repeat(65)).validated().is_err());
    }

    #[test]
    fn invalid_preferences_are_refused() {
        let mut bad = profile("A");
        bad.preferences.global.switch_delay_seconds = 255;
        assert!(bad.validated().is_err());
    }

    #[test]
    fn saving_replaces_by_name_and_deleting_reports_whether_it_existed() {
        let mut list = Vec::new();
        save(&mut list, profile("A")).unwrap();
        let mut changed = profile("A");
        changed.disabled.insert("Roblox".into());
        save(&mut list, changed).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].disabled.contains("Roblox"));
        assert!(delete(&mut list, "A"));
        assert!(!delete(&mut list, "A"));
    }

    #[test]
    fn the_list_has_a_limit_but_replacing_still_works() {
        let mut list = Vec::new();
        for number in 0..MAX_PROFILES {
            save(&mut list, profile(&format!("P{number}"))).unwrap();
        }
        assert!(save(&mut list, profile("one more")).is_err());
        assert!(save(&mut list, profile("P0")).is_ok());
    }
}
