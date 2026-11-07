//! Checks and normalization for preferences that come from the interface or
//! from a file edited by hand.

use crate::policy::Preferences;

/// Most entries in each exclusion list.
const MAX_LIST: usize = 100;
/// Most presences that can carry their own preferences.
const MAX_PRESENCES: usize = 4096;
/// Longest switch delay the interface offers, in seconds.
const MAX_DELAY: u32 = 30;
const MAX_PRIORITY: u8 = 100;
/// Range of the idle setting, in minutes.
const IDLE_MINUTES: std::ops::RangeInclusive<u32> = 5..=120;
/// Longest time paused media stays on display, in minutes.
const MAX_PAUSED_MINUTES: u32 = 60;
/// First minute past the end of a day.
const MINUTES_IN_A_DAY: u16 = 24 * 60;

/// Why a set of preferences was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreferenceError {
    /// The switch delay is above the maximum.
    #[error("switch delay must be at most {MAX_DELAY} seconds")]
    Delay,
    /// The idle time is outside the range the interface offers.
    #[error("the idle time must be between 5 and 120 minutes")]
    Idle,
    /// An hour of the day past midnight.
    #[error("hours must fall inside the day")]
    Hours,
    /// Paused media would stay longer than the interface offers.
    #[error("paused media can stay at most {MAX_PAUSED_MINUTES} minutes")]
    PausedTime,
    /// A priority is above the maximum.
    #[error("priority of {0} must be between 0 and {MAX_PRIORITY}")]
    Priority(String),
    /// A list or map has too many entries.
    #[error("{0} has too many entries")]
    TooMany(&'static str),
    /// An excluded domain is not a plain hostname.
    #[error("{0:?} is not a hostname; use names such as example.com, without paths or wildcards")]
    Domain(String),
    /// An excluded application name is empty or too long.
    #[error("application names must have 1 to 253 characters")]
    Application,
}

/// Lowercase hostname made of letters, digits, dots and hyphens.
fn hostname(text: &str) -> Option<String> {
    let domain = text.trim().to_lowercase();
    let valid = !domain.is_empty()
        && domain.len() <= 253
        && domain.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        });
    valid.then_some(domain)
}

/// Drops repeated entries, keeping the first.
fn unique(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

/// Trims, lowers and deduplicates a list of program names.
fn program_list(names: Vec<String>) -> Result<Vec<String>, PreferenceError> {
    let names = names
        .into_iter()
        .map(|name| {
            let name = name.trim().to_lowercase();
            (1..=253)
                .contains(&name.len())
                .then_some(name)
                .ok_or(PreferenceError::Application)
        })
        .collect::<Result<_, _>>()?;
    Ok(unique(names))
}

impl Preferences {
    /// Checks every value and normalizes the exclusion lists.
    ///
    /// Raises:
    ///     PreferenceError: A value is out of range or a list is too long.
    pub fn validated(mut self) -> Result<Self, PreferenceError> {
        let global = &mut self.global;
        if global.switch_delay_seconds > MAX_DELAY {
            return Err(PreferenceError::Delay);
        }
        if global
            .idle_minutes
            .is_some_and(|minutes| !IDLE_MINUTES.contains(&minutes))
        {
            return Err(PreferenceError::Idle);
        }
        if global.paused_minutes > MAX_PAUSED_MINUTES {
            return Err(PreferenceError::PausedTime);
        }
        if global
            .quiet_hours
            .is_some_and(|hours| hours.from >= MINUTES_IN_A_DAY || hours.to >= MINUTES_IN_A_DAY)
        {
            return Err(PreferenceError::Hours);
        }
        if global.quiet_programs.len() > MAX_LIST {
            return Err(PreferenceError::TooMany("programs that hide everything"));
        }
        if global.excluded_domains.len() > MAX_LIST {
            return Err(PreferenceError::TooMany("excluded domains"));
        }
        if global.excluded_applications.len() > MAX_LIST {
            return Err(PreferenceError::TooMany("excluded applications"));
        }
        let domains = std::mem::take(&mut global.excluded_domains);
        global.excluded_domains = unique(
            domains
                .into_iter()
                .map(|domain| hostname(&domain).ok_or(PreferenceError::Domain(domain)))
                .collect::<Result<_, _>>()?,
        );
        let applications = std::mem::take(&mut global.excluded_applications);
        global.excluded_applications = program_list(applications)?;
        let programs = std::mem::take(&mut global.quiet_programs);
        global.quiet_programs = program_list(programs)?;
        if self.presences.len() > MAX_PRESENCES {
            return Err(PreferenceError::TooMany("presence preferences"));
        }
        for (name, options) in &self.presences {
            if options
                .priority
                .is_some_and(|priority| priority > MAX_PRIORITY)
            {
                return Err(PreferenceError::Priority(name.clone()));
            }
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::PresencePreferences;

    #[test]
    fn defaults_are_valid_and_unchanged() {
        let defaults = Preferences::default();
        assert_eq!(defaults.clone().validated().unwrap(), defaults);
    }

    #[test]
    fn lists_are_trimmed_lowercased_and_deduplicated() {
        let mut preferences = Preferences::default();
        preferences.global.excluded_domains =
            vec![" Example.COM ".into(), "example.com".into(), "b.io".into()];
        preferences.global.excluded_applications = vec!["Game.EXE".into(), "game.exe".into()];
        let clean = preferences.validated().unwrap();
        assert_eq!(clean.global.excluded_domains, ["example.com", "b.io"]);
        assert_eq!(clean.global.excluded_applications, ["game.exe"]);
    }

    #[test]
    fn domains_must_be_plain_hostnames() {
        for bad in [
            "",
            "a b.com",
            "https://a.com",
            "a.com/path",
            "*.a.com",
            "a..com",
            "-a.com",
            "a.com.",
        ] {
            let mut preferences = Preferences::default();
            preferences.global.excluded_domains = vec![bad.into()];
            assert_eq!(
                preferences.validated(),
                Err(PreferenceError::Domain(bad.into())),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn paused_media_stays_at_most_an_hour() {
        let mut preferences = Preferences::default();
        preferences.global.paused_minutes = 61;
        assert_eq!(preferences.validated(), Err(PreferenceError::PausedTime));
        let mut preferences = Preferences::default();
        preferences.global.paused_minutes = 60;
        assert!(preferences.validated().is_ok());
    }

    #[test]
    fn the_idle_time_stays_in_range() {
        for minutes in [0, 4, 121, u32::MAX] {
            let mut preferences = Preferences::default();
            preferences.global.idle_minutes = Some(minutes);
            assert_eq!(
                preferences.validated(),
                Err(PreferenceError::Idle),
                "{minutes}"
            );
        }
        for minutes in [None, Some(5), Some(120)] {
            let mut preferences = Preferences::default();
            preferences.global.idle_minutes = minutes;
            assert!(preferences.validated().is_ok(), "{minutes:?}");
        }
    }

    #[test]
    fn limits_are_enforced() {
        let mut slow = Preferences::default();
        slow.global.switch_delay_seconds = 31;
        assert_eq!(slow.validated(), Err(PreferenceError::Delay));

        let mut long = Preferences::default();
        long.global.excluded_domains = (0..101).map(|n| format!("d{n}.com")).collect();
        assert!(matches!(long.validated(), Err(PreferenceError::TooMany(_))));

        let mut high = Preferences::default();
        high.presences.insert(
            "Game".into(),
            PresencePreferences {
                priority: Some(101),
                ..Default::default()
            },
        );
        assert_eq!(
            high.validated(),
            Err(PreferenceError::Priority("Game".into()))
        );

        let mut empty = Preferences::default();
        empty.global.excluded_applications = vec!["  ".into()];
        assert_eq!(empty.validated(), Err(PreferenceError::Application));
    }

    #[test]
    fn quiet_rules_are_checked_and_normalized() {
        let mut preferences = Preferences::default();
        preferences.global.quiet_programs = vec![" OBS64.exe ".into(), "obs64.exe".into()];
        preferences.global.quiet_hours = Some(crate::policy::QuietHours {
            from: 1320,
            to: 420,
        });
        let saved = preferences.validated().unwrap();
        assert_eq!(saved.global.quiet_programs, ["obs64.exe"]);

        let mut late = Preferences::default();
        late.global.quiet_hours = Some(crate::policy::QuietHours { from: 0, to: 1440 });
        assert_eq!(late.validated(), Err(PreferenceError::Hours));

        let mut many = Preferences::default();
        many.global.quiet_programs = vec!["a".into(); MAX_LIST + 1];
        assert!(matches!(many.validated(), Err(PreferenceError::TooMany(_))));
    }
}
