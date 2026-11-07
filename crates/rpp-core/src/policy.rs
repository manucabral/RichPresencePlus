//! Choosing which activity to show and removing what the user wants hidden.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::activity::{Activity, MAX_TEXT, StatusDisplay};

/// Group an activity belongs to; each group has a default priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Activity typed by the user, always wins.
    Manual,
    /// A game.
    Game,
    /// Video playback.
    Video,
    /// Music playback.
    Music,
    /// Anything else.
    Other,
}

impl Kind {
    /// Priority from 0 to 100 used when the user has not set one.
    pub fn default_priority(self) -> u8 {
        match self {
            Self::Manual => 100,
            Self::Game => 80,
            Self::Video => 60,
            Self::Music => 40,
            Self::Other => 20,
        }
    }
}

/// How much of an activity leaves the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrivacyMode {
    /// Everything the presence reports.
    #[default]
    Full,
    /// Only the name of the service, no titles, images, times or buttons.
    Service,
}

/// Rules that apply to every presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalPreferences {
    /// Publish nothing at all.
    pub paused: bool,
    /// Amount of detail that is published.
    pub privacy: PrivacyMode,
    /// Hostnames whose subdomains are excluded too, in lowercase.
    pub excluded_domains: Vec<String>,
    /// Application names, in lowercase.
    pub excluded_applications: Vec<String>,
    /// Seconds to wait before a lower-ranked change replaces the current one.
    pub switch_delay_seconds: u32,
    /// Hide every activity after this many minutes without keyboard or mouse
    /// input; off when unset.
    pub idle_minutes: Option<u32>,
    /// Minutes a paused source stays on display, without its counters; 0 hides
    /// it at once.
    pub paused_minutes: u32,
    /// What started playing last wins before priority; manual activities
    /// still come first.
    pub prefer_recent: bool,
    /// Programs that hide every activity while they run, in lowercase.
    pub quiet_programs: Vec<String>,
    /// Hours of the day when every activity is hidden; off when unset.
    pub quiet_hours: Option<QuietHours>,
}

/// A stretch of the day, in minutes after midnight on the computer's clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuietHours {
    /// First minute it covers.
    pub from: u16,
    /// First minute it no longer covers.
    pub to: u16,
}

impl QuietHours {
    /// True when `minute`, counted from midnight, falls inside. Equal ends
    /// cover nothing.
    pub fn covers(self, minute: u16) -> bool {
        if self.from <= self.to {
            (self.from..self.to).contains(&minute)
        } else {
            minute >= self.from || minute < self.to
        }
    }
}

impl Default for GlobalPreferences {
    fn default() -> Self {
        Self {
            paused: false,
            privacy: PrivacyMode::Full,
            excluded_domains: Vec::new(),
            excluded_applications: Vec::new(),
            switch_delay_seconds: 3,
            idle_minutes: None,
            paused_minutes: 0,
            prefer_recent: false,
            quiet_programs: Vec::new(),
            quiet_hours: None,
        }
    }
}

/// Rules that apply to one presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresencePreferences {
    /// Allow this presence to publish.
    pub enabled: bool,
    /// Overrides the group priority when set.
    pub priority: Option<u8>,
    /// Keep titles and captions.
    pub show_title: bool,
    /// Keep buttons.
    pub show_buttons: bool,
    /// Keep the elapsed and remaining counters.
    pub show_timestamps: bool,
    /// Which text the member list shows, replacing the presence's own choice.
    pub status_display: Option<StatusDisplay>,
    /// Hide while paused (after the global grace time), replacing the
    /// presence's own wish; `false` keeps it on display while paused.
    pub hide_when_paused: Option<bool>,
}

impl Default for PresencePreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: None,
            show_title: true,
            show_buttons: true,
            show_timestamps: true,
            status_display: None,
            hide_when_paused: None,
        }
    }
}

/// All publishing preferences.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// Rules for every presence.
    pub global: GlobalPreferences,
    /// Rules by presence name.
    pub presences: HashMap<String, PresencePreferences>,
}

/// An activity offered by one source, with what the policy needs to judge it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Stable identifier of the presence.
    pub source: String,
    /// Name shown to the user and used when details are hidden.
    pub label: String,
    /// Discord application the activity is published under.
    pub client_id: String,
    /// The activity itself.
    pub activity: Activity,
    /// Group used to rank it.
    pub kind: Kind,
    /// The source is in the foreground.
    pub foreground: bool,
    /// The media is paused.
    pub paused: bool,
    /// The presence's own wish to hide while paused; the user can change it.
    pub hide_when_paused: bool,
    /// Hostnames involved, for the exclusion list.
    pub domains: Vec<String>,
    /// Application name, for the exclusion list.
    pub application: String,
    /// Moment after which the source counts as silent.
    pub expires_at: Option<Instant>,
}

/// One line of the explanation shown to the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// Presence identifier.
    pub source: String,
    /// Presence name.
    pub label: String,
    /// Priority after the user's override.
    pub priority: u8,
    /// Group of the presence.
    pub kind: Kind,
    /// Why it was or was not chosen.
    pub reason: String,
    /// True for the one on display.
    pub selected: bool,
}

/// Result of one selection round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// Identifier of the chosen presence.
    pub winner: Option<String>,
    /// What to publish, already filtered by the privacy rules.
    pub activity: Option<Activity>,
    /// An explanation for every candidate.
    pub rows: Vec<Row>,
}

/// Lowercase hostname of an address, empty when the text is not one.
pub fn hostname(address: &str) -> String {
    Url::parse(address)
        .ok()
        .and_then(|url| url.host_str().map(str::to_lowercase))
        .unwrap_or_default()
}

/// A domain as a script or an address wrote it, ready to compare with the list
/// the user keeps: without case, credentials, port, final dot or `www.`.
fn bare_domain(domain: &str) -> String {
    let lower = domain.trim().to_lowercase();
    let host = lower.rsplit('@').next().unwrap_or_default();
    let host = match host.rsplit_once(':') {
        Some((name, port)) if port.bytes().all(|byte| byte.is_ascii_digit()) => name,
        _ => host,
    };
    let host = host.trim_end_matches('.');
    host.strip_prefix("www.").unwrap_or(host).to_owned()
}

fn domain_excluded(domain: &str, excluded: &[String]) -> bool {
    let domain = bare_domain(domain);
    excluded.iter().any(|item| {
        let item = bare_domain(item);
        domain == item || domain.ends_with(&format!(".{item}"))
    })
}

fn exclusion_reason(
    candidate: &Candidate,
    global: &GlobalPreferences,
    options: &PresencePreferences,
) -> Option<&'static str> {
    if global.paused {
        return Some("Publishing is paused");
    }
    if !options.enabled {
        return Some("Disabled for this presence");
    }
    let identifiers = [&candidate.source, &candidate.label, &candidate.application];
    if identifiers
        .iter()
        .any(|value| global.excluded_applications.contains(&value.to_lowercase()))
    {
        return Some("Application excluded by privacy preferences");
    }
    // Every link counts, not only buttons: a clickable title opens the page too.
    let link_hosts = candidate.activity.links().map(hostname);
    let blocked = candidate
        .domains
        .iter()
        .cloned()
        .chain(link_hosts)
        .any(|domain| domain_excluded(&domain, &global.excluded_domains));
    if blocked {
        return Some("Domain excluded by privacy preferences");
    }
    None
}

/// Why a paused source is not on display.
pub const PAUSED_REASON: &str = "Media is paused";

fn shorten(label: &str) -> String {
    label.chars().take(MAX_TEXT).collect()
}

/// A copy of the activity without the fields the user chose not to publish.
pub fn filter_activity(
    candidate: &Candidate,
    global: &GlobalPreferences,
    options: &PresencePreferences,
) -> Activity {
    let mut activity = candidate.activity.clone();
    // A script writes `name`, so hiding the title must reach it too.
    let service_name = activity.name.as_ref().map(|_| shorten(&candidate.label));
    if global.privacy == PrivacyMode::Service {
        return Activity {
            name: service_name,
            details: Some(shorten(&candidate.label)),
            kind: activity.kind,
            ..Activity::default()
        };
    }
    if !options.show_title {
        activity.name = service_name;
        activity.details = Some(shorten(&candidate.label));
        activity.state = None;
        activity.large_text = None;
        activity.small_text = None;
        // The links lead to what the title named.
        activity.clear_links();
    }
    if !options.show_buttons {
        // Links on texts and images are buttons by another name.
        activity.buttons.clear();
        activity.clear_links();
    }
    if !options.show_timestamps {
        activity.start = None;
        activity.end = None;
    }
    if let Some(display) = options.status_display {
        activity.status_display = Some(display);
    }
    activity.drop_orphans();
    activity
}

/// Why a source is hidden while the computer is idle.
pub const IDLE_REASON: &str = "Hidden while the computer is idle";

/// True when the time without keyboard or mouse input reached the user's
/// setting. An unknown time or a `None` setting never counts as idle.
pub fn is_idle(idle_for: Option<Duration>, minutes: Option<u32>) -> bool {
    match (idle_for, minutes) {
        (Some(idle_for), Some(minutes)) => idle_for >= Duration::from_secs(u64::from(minutes) * 60),
        _ => false,
    }
}

/// Why nothing shows while a rule of the person holds.
pub const QUIET_REASON: &str = "Hidden by your automatic rules";

fn program_name(name: &str) -> String {
    let name = name.trim().to_lowercase();
    name.strip_suffix(".exe").unwrap_or(&name).to_owned()
}

/// True when a listed program runs or the hour falls inside the quiet hours.
///
/// Args:
///     minute: Minutes after midnight; `None`, when the clock cannot be read,
///         leaves the hours out.
pub fn is_quiet(global: &GlobalPreferences, running: &[String], minute: Option<u16>) -> bool {
    let hour = global
        .quiet_hours
        .zip(minute)
        .is_some_and(|(hours, minute)| hours.covers(minute));
    hour || running.iter().any(|name| {
        let name = program_name(name);
        global
            .quiet_programs
            .iter()
            .any(|listed| program_name(listed) == name)
    })
}

/// Keeps the current choice stable and delays ordinary switches.
#[derive(Debug, Default)]
pub struct Selector {
    current: Option<String>,
    pending: Option<(String, Instant)>,
    idle: bool,
    quiet: bool,
    /// When each source still paused was first seen paused.
    paused_since: HashMap<String, Instant>,
    /// The earliest moment a paused source on its grace time must go.
    paused_until: Option<Instant>,
    /// When each source still playing started, or played again after a pause.
    playing_since: HashMap<String, Instant>,
}

/// What decides between eligible sources. Fields compare in the order they
/// are written and the greatest wins: a manual activity, then one that plays,
/// then the most recent start (only when the user asks for it), then
/// priority, focus, and being on display already, so equals never flip-flop.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Rank<'a> {
    manual: bool,
    playing: bool,
    recent: Option<Instant>,
    priority: u8,
    foreground: bool,
    current: bool,
    source: &'a str,
}

impl Selector {
    /// True while the selection can change with time alone: a switch waits for
    /// its delay, or a paused source is on its grace time. The caller should
    /// select again soon.
    pub fn has_pending(&self) -> bool {
        self.pending.is_some() || self.paused_until.is_some()
    }

    /// Records whether the computer is idle; while it is, nothing is selected.
    ///
    /// Returns:
    ///     True when the state changed and the selection must run again.
    pub fn set_idle(&mut self, idle: bool) -> bool {
        std::mem::replace(&mut self.idle, idle) != idle
    }

    /// Records whether a quiet rule hides everything now.
    ///
    /// Returns:
    ///     True when the state changed and the selection must run again.
    pub fn set_quiet(&mut self, quiet: bool) -> bool {
        std::mem::replace(&mut self.quiet, quiet) != quiet
    }

    /// Picks the activity to show among the candidates.
    pub fn select(
        &mut self,
        candidates: &[Candidate],
        preferences: &Preferences,
        now: Instant,
    ) -> Selection {
        let defaults = PresencePreferences::default();
        let mut rows = Vec::with_capacity(candidates.len());
        let mut eligible: Vec<Rank> = Vec::new();
        let grace = Duration::from_secs(u64::from(preferences.global.paused_minutes) * 60);
        let mut paused_until: Option<Instant> = None;
        self.paused_since
            .retain(|source, _| candidates.iter().any(|c| c.paused && &c.source == source));
        // A pause, or leaving, ends a start; playing again is a new one.
        self.playing_since
            .retain(|source, _| candidates.iter().any(|c| !c.paused && &c.source == source));
        for candidate in candidates.iter().filter(|c| !c.paused) {
            self.playing_since
                .entry(candidate.source.clone())
                .or_insert(now);
        }

        for candidate in candidates {
            let options = preferences
                .presences
                .get(&candidate.source)
                .unwrap_or(&defaults);
            let mut reason = exclusion_reason(candidate, &preferences.global, options)
                .map(str::to_owned)
                .unwrap_or_default();
            let hides = options
                .hide_when_paused
                .unwrap_or(candidate.hide_when_paused);
            if reason.is_empty() && candidate.paused && hides {
                let since = *self
                    .paused_since
                    .entry(candidate.source.clone())
                    .or_insert(now);
                let ends = since + grace;
                if now < ends {
                    paused_until = Some(paused_until.map_or(ends, |at| at.min(ends)));
                } else {
                    reason = PAUSED_REASON.into();
                }
            }
            // Idle hides everything, manual activities included: the user is away.
            if reason.is_empty() && self.idle {
                reason = IDLE_REASON.into();
            }
            // The same for a rule: a stream must not show even a custom status.
            if reason.is_empty() && self.quiet {
                reason = QUIET_REASON.into();
            }
            if reason.is_empty() && candidate.expires_at.is_some_and(|at| now >= at) {
                reason = "Presence stopped reporting".into();
            }
            let priority = options
                .priority
                .unwrap_or_else(|| candidate.kind.default_priority());
            if reason.is_empty() {
                let recent = preferences
                    .global
                    .prefer_recent
                    .then(|| self.playing_since.get(&candidate.source).copied())
                    .flatten();
                eligible.push(Rank {
                    manual: candidate.kind == Kind::Manual,
                    playing: !candidate.paused,
                    recent,
                    priority,
                    foreground: candidate.foreground,
                    current: self.current.as_deref() == Some(candidate.source.as_str()),
                    source: &candidate.source,
                });
            }
            rows.push(Row {
                source: candidate.source.clone(),
                label: candidate.label.clone(),
                priority,
                kind: candidate.kind,
                reason,
                selected: false,
            });
        }

        let mut winner = eligible.iter().max().map(|best| best.source.to_owned());
        let current_eligible = self
            .current
            .as_deref()
            .is_some_and(|name| eligible.iter().any(|item| item.source == name));

        if winner != self.current
            && current_eligible
            && let Some(next) = winner.clone()
        {
            let waiting = matches!(&self.pending, Some((name, _)) if *name == next);
            if !waiting {
                self.pending = Some((next, now));
            }
            let since = self.pending.as_ref().map_or(now, |(_, since)| *since);
            let delay = Duration::from_secs(preferences.global.switch_delay_seconds.into());
            if now.saturating_duration_since(since) < delay {
                winner = self.current.clone();
            }
        }

        let pending_name = self.pending.as_ref().map(|(name, _)| name.as_str());
        let pending_eligible =
            pending_name.is_some_and(|name| eligible.iter().any(|i| i.source == name));
        if winner.is_none()
            || winner.as_deref() == pending_name
            || (winner == self.current && !pending_eligible)
        {
            self.pending = None;
        }
        self.current.clone_from(&winner);

        let waiting_for = self.pending.as_ref().map(|(name, _)| name.clone());
        for row in &mut rows {
            if winner.as_deref() == Some(row.source.as_str()) {
                row.selected = true;
                row.reason = format!("Selected: {:?}, priority {}", row.kind, row.priority);
            } else if row.reason.is_empty() {
                row.reason = if waiting_for.as_deref() == Some(row.source.as_str()) {
                    "Waiting before switching".into()
                } else {
                    "Another activity has a higher priority; on a tie the current one stays".into()
                };
            }
        }

        self.paused_until = paused_until;
        let activity = winner.as_deref().and_then(|name| {
            let candidate = candidates.iter().find(|c| c.source == name)?;
            let options = preferences.presences.get(name).unwrap_or(&defaults);
            let mut activity = filter_activity(candidate, &preferences.global, options);
            if candidate.paused {
                // The counters would keep running while nothing plays.
                activity.start = None;
                activity.end = None;
            }
            Some(activity)
        });
        Selection {
            winner,
            activity,
            rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::Button;

    fn candidate(source: &str, kind: Kind) -> Candidate {
        Candidate {
            source: source.into(),
            label: source.into(),
            client_id: "123456789012345678".into(),
            activity: Activity {
                details: Some(format!("{source} details")),
                state: Some("In progress".into()),
                start: Some(100),
                buttons: vec![Button {
                    label: "Open".into(),
                    url: "https://www.example.com/a?token=1".into(),
                }],
                ..Activity::default()
            },
            kind,
            foreground: false,
            paused: false,
            hide_when_paused: true,
            domains: Vec::new(),
            application: String::new(),
            expires_at: None,
        }
    }

    fn instant_after(base: Instant, seconds: u64) -> Instant {
        base + Duration::from_secs(seconds)
    }

    #[test]
    fn higher_group_wins_and_user_priority_overrides() {
        let mut selector = Selector::default();
        let now = Instant::now();
        let items = [
            candidate("Song", Kind::Music),
            candidate("Game", Kind::Game),
        ];
        let mut preferences = Preferences::default();
        assert_eq!(
            selector.select(&items, &preferences, now).winner.as_deref(),
            Some("Game")
        );

        let mut selector = Selector::default();
        preferences.presences.insert(
            "Song".into(),
            PresencePreferences {
                priority: Some(95),
                ..Default::default()
            },
        );
        assert_eq!(
            selector.select(&items, &preferences, now).winner.as_deref(),
            Some("Song")
        );
    }

    #[test]
    fn ties_keep_the_current_activity() {
        let mut selector = Selector::default();
        let now = Instant::now();
        let preferences = Preferences::default();
        let first = [candidate("Alpha", Kind::Video)];
        selector.select(&first, &preferences, now);
        let both = [
            candidate("Alpha", Kind::Video),
            candidate("Zulu", Kind::Video),
        ];
        let result = selector.select(&both, &preferences, instant_after(now, 10));
        assert_eq!(result.winner.as_deref(), Some("Alpha"));
    }

    #[test]
    fn switch_waits_for_the_delay() {
        let mut selector = Selector::default();
        let now = Instant::now();
        let preferences = Preferences::default();
        selector.select(&[candidate("Song", Kind::Music)], &preferences, now);
        let both = [
            candidate("Song", Kind::Music),
            candidate("Game", Kind::Game),
        ];

        let early = selector.select(&both, &preferences, instant_after(now, 1));
        assert_eq!(early.winner.as_deref(), Some("Song"));
        assert!(selector.has_pending());
        assert!(
            early
                .rows
                .iter()
                .any(|r| r.reason == "Waiting before switching")
        );

        let late = selector.select(&both, &preferences, instant_after(now, 5));
        assert_eq!(late.winner.as_deref(), Some("Game"));
        assert!(!selector.has_pending());
    }

    #[test]
    fn exclusion_clears_without_waiting() {
        let mut selector = Selector::default();
        let now = Instant::now();
        let mut preferences = Preferences::default();
        let mut item = candidate("Video", Kind::Video);
        item.domains = vec!["video.example.com".into()];
        selector.select(std::slice::from_ref(&item), &preferences, now);

        preferences.global.excluded_domains = vec!["example.com".into()];
        let result = selector.select(std::slice::from_ref(&item), &preferences, now);
        assert_eq!(result.winner, None);
        assert_eq!(
            result.rows[0].reason,
            "Domain excluded by privacy preferences"
        );
    }

    #[test]
    fn an_exclusion_holds_however_a_script_writes_the_domain() {
        let mut preferences = Preferences::default();
        preferences.global.excluded_domains = vec!["blocked.test".into()];
        let now = Instant::now();
        for domain in [
            "blocked.test:8443",
            "blocked.test.",
            "BLOCKED.test",
            "video.blocked.test.:443",
            "user:secret@blocked.test",
        ] {
            let mut item = candidate("Video", Kind::Video);
            item.domains = vec![domain.into()];
            let result = Selector::default().select(&[item], &preferences, now);
            assert_eq!(result.winner, None, "{domain} got through");
        }
        let mut item = candidate("Video", Kind::Video);
        item.domains = vec!["notblocked.test:8443".into()];
        let shown = Selector::default().select(&[item], &preferences, now);
        assert!(shown.winner.is_some());
    }

    #[test]
    fn an_exclusion_holds_with_or_without_www_on_either_side() {
        let now = Instant::now();
        for (excluded, reported) in [
            ("www.blocked.test", "blocked.test"),
            ("blocked.test", "www.blocked.test"),
            ("www.blocked.test", "www.blocked.test"),
            ("www.blocked.test", "music.blocked.test"),
        ] {
            let mut preferences = Preferences::default();
            preferences.global.excluded_domains = vec![excluded.into()];
            let mut item = candidate("Video", Kind::Video);
            item.domains = vec![reported.into()];
            let result = Selector::default().select(&[item], &preferences, now);
            assert_eq!(result.winner, None, "{reported} passed {excluded}");
        }
    }

    #[test]
    fn button_hosts_count_for_exclusions() {
        let mut selector = Selector::default();
        let mut preferences = Preferences::default();
        preferences.global.excluded_domains = vec!["example.com".into()];
        let item = candidate("Video", Kind::Video);
        let result = selector.select(&[item], &preferences, Instant::now());
        assert_eq!(result.winner, None);
    }

    #[test]
    fn paused_media_and_stopped_sources_are_skipped() {
        let mut selector = Selector::default();
        let now = Instant::now();
        let preferences = Preferences::default();
        let mut paused = candidate("Paused", Kind::Video);
        paused.paused = true;
        let mut stale = candidate("Stale", Kind::Game);
        stale.expires_at = Some(now);
        let result = selector.select(&[paused, stale], &preferences, now);
        assert_eq!(result.winner, None);
        assert_eq!(result.rows[0].reason, "Media is paused");
        assert_eq!(result.rows[1].reason, "Presence stopped reporting");
    }

    #[test]
    fn service_mode_keeps_only_the_name() {
        let mut selector = Selector::default();
        let mut preferences = Preferences::default();
        preferences.global.privacy = PrivacyMode::Service;
        let result = selector.select(
            &[candidate("Video", Kind::Video)],
            &preferences,
            Instant::now(),
        );
        let activity = result.activity.unwrap();
        assert_eq!(activity.details.as_deref(), Some("Video"));
        assert_eq!(activity.state, None);
        assert_eq!(activity.start, None);
        assert!(activity.buttons.is_empty());
        assert_eq!(activity.links().count(), 0);
        assert_eq!(activity.party_size, None);
    }

    fn text_links(activity: &Activity) -> usize {
        [
            &activity.details_url,
            &activity.state_url,
            &activity.large_url,
            &activity.small_url,
        ]
        .iter()
        .filter(|link| link.is_some())
        .count()
    }

    fn linked(name: &str) -> Candidate {
        let mut item = candidate(name, Kind::Music);
        item.activity.details = Some("Song".into());
        item.activity.state = Some("Artist".into());
        item.activity.large_image = Some("cover".into());
        item.activity.details_url = Some("https://music.example/track/1".into());
        item.activity.state_url = Some("https://music.example/artist/1".into());
        item.activity.large_url = Some("https://music.example/album/1".into());
        item.activity.status_display = Some(crate::activity::StatusDisplay::State);
        item.activity.party_size = Some([2, 4]);
        item
    }

    #[test]
    fn hiding_the_title_or_the_buttons_removes_every_link() {
        let item = linked("Music");
        let all = filter_activity(
            &item,
            &GlobalPreferences::default(),
            &PresencePreferences::default(),
        );
        // The helper's button and the three links.
        assert_eq!(all.links().count(), 4);
        assert_eq!(
            all.status_display,
            Some(crate::activity::StatusDisplay::State)
        );

        let untitled = PresencePreferences {
            show_title: false,
            ..Default::default()
        };
        let activity = filter_activity(&item, &GlobalPreferences::default(), &untitled);
        assert_eq!(text_links(&activity), 0);
        assert_eq!(activity.buttons.len(), 1);
        // The state line is gone, so the member list cannot show it.
        assert_eq!(activity.status_display, None);
        assert_eq!(activity.party_size, Some([2, 4]));

        let unbuttoned = PresencePreferences {
            show_buttons: false,
            ..Default::default()
        };
        let activity = filter_activity(&item, &GlobalPreferences::default(), &unbuttoned);
        assert_eq!(activity.links().count(), 0);
        assert_eq!(activity.state.as_deref(), Some("Artist"));
    }

    #[test]
    fn an_excluded_domain_in_any_link_hides_the_activity() {
        let mut selector = Selector::default();
        let mut preferences = Preferences::default();
        preferences.global.excluded_domains = vec!["music.example".into()];
        let mut item = linked("Music");
        item.activity.details_url = None;
        item.activity.state_url = None;
        // Only the image links to the excluded site.
        let result = selector.select(&[item], &preferences, Instant::now());
        assert_eq!(result.winner, None);
        assert_eq!(
            result.rows[0].reason,
            "Domain excluded by privacy preferences"
        );
    }

    #[test]
    fn display_options_remove_only_their_fields() {
        let item = candidate("Video", Kind::Video);
        let options = PresencePreferences {
            show_title: false,
            show_buttons: false,
            show_timestamps: false,
            ..Default::default()
        };
        let activity = filter_activity(&item, &GlobalPreferences::default(), &options);
        assert_eq!(activity.details.as_deref(), Some("Video"));
        assert_eq!(activity.state, None);
        assert_eq!(activity.start, None);
        assert!(activity.buttons.is_empty());
    }

    fn paused(source: &str, kind: Kind) -> Candidate {
        let mut item = candidate(source, kind);
        item.paused = true;
        item
    }

    #[test]
    fn paused_media_stays_for_its_grace_time_without_counters() {
        let mut selector = Selector::default();
        let mut preferences = Preferences::default();
        preferences.global.paused_minutes = 5;
        let base = Instant::now();
        let song = [paused("Music", Kind::Music)];

        let result = selector.select(&song, &preferences, base);
        assert_eq!(result.winner.as_deref(), Some("Music"));
        let shown = result.activity.unwrap();
        assert_eq!((shown.start, shown.end), (None, None));
        assert!(selector.has_pending(), "the grace time must be watched");

        let later = base + Duration::from_secs(5 * 60 - 1);
        assert_eq!(
            selector
                .select(&song, &preferences, later)
                .winner
                .as_deref(),
            Some("Music")
        );
        let over = base + Duration::from_secs(5 * 60);
        let result = selector.select(&song, &preferences, over);
        assert_eq!(result.winner, None);
        assert_eq!(result.rows[0].reason, PAUSED_REASON);
        assert!(!selector.has_pending());
    }

    #[test]
    fn playing_again_restarts_the_grace_time() {
        let mut selector = Selector::default();
        let mut preferences = Preferences::default();
        preferences.global.paused_minutes = 1;
        let base = Instant::now();
        selector.select(&[paused("Music", Kind::Music)], &preferences, base);
        let resumed = base + Duration::from_secs(50);
        selector.select(&[candidate("Music", Kind::Music)], &preferences, resumed);
        // Paused again 50 s in: a fresh minute, not the ten seconds left.
        let again = base + Duration::from_secs(55);
        selector.select(&[paused("Music", Kind::Music)], &preferences, again);
        let check = base + Duration::from_secs(100);
        let result = selector.select(&[paused("Music", Kind::Music)], &preferences, check);
        assert_eq!(result.winner.as_deref(), Some("Music"));
    }

    #[test]
    fn a_paused_source_gives_way_to_one_that_plays() {
        let mut selector = Selector::default();
        let mut preferences = Preferences::default();
        preferences.global.paused_minutes = 10;
        preferences.global.switch_delay_seconds = 0;
        let offered = [paused("Game", Kind::Game), candidate("Music", Kind::Music)];
        let result = selector.select(&offered, &preferences, Instant::now());
        assert_eq!(result.winner.as_deref(), Some("Music"));
    }

    #[test]
    fn a_presence_can_stay_while_paused_and_the_user_decides_last() {
        let mut selector = Selector::default();
        let mut preferences = Preferences::default();
        let base = Instant::now();
        let mut song = paused("Music", Kind::Music);
        // The developer keeps it while paused; the global grace time is 0.
        song.hide_when_paused = false;
        let later = base + Duration::from_secs(3 * 3600);
        let result = selector.select(&[song.clone()], &preferences, later);
        assert_eq!(result.winner.as_deref(), Some("Music"));
        assert_eq!(result.activity.unwrap().start, None);
        // Nothing to wait for, so no extra selections.
        assert!(!selector.has_pending());

        // The user turns hiding on for this presence.
        preferences.presences.insert(
            "Music".into(),
            PresencePreferences {
                hide_when_paused: Some(true),
                ..Default::default()
            },
        );
        let result = selector.select(&[song.clone()], &preferences, later);
        assert_eq!(result.winner, None);
        assert_eq!(result.rows[0].reason, PAUSED_REASON);

        // And off again for a presence whose developer hides it.
        let mut hidden = paused("Video", Kind::Video);
        hidden.hide_when_paused = true;
        preferences.presences.insert(
            "Video".into(),
            PresencePreferences {
                hide_when_paused: Some(false),
                ..Default::default()
            },
        );
        let result = selector.select(&[hidden], &preferences, later);
        assert_eq!(result.winner.as_deref(), Some("Video"));
    }

    #[test]
    fn the_last_thing_started_wins_when_the_user_asks_for_it() {
        let mut selector = Selector::default();
        let mut preferences = Preferences::default();
        preferences.global.switch_delay_seconds = 0;
        let base = Instant::now();
        let video = candidate("YouTube", Kind::Video);
        let music = candidate("YouTube Music", Kind::Music);

        // Without the option, the higher priority wins whoever started last.
        selector.select(std::slice::from_ref(&video), &preferences, base);
        let later = base + Duration::from_secs(5);
        let both = [video.clone(), music.clone()];
        assert_eq!(
            selector
                .select(&both, &preferences, later)
                .winner
                .as_deref(),
            Some("YouTube")
        );

        // With it, the music started later and wins.
        preferences.global.prefer_recent = true;
        assert_eq!(
            selector
                .select(&both, &preferences, later)
                .winner
                .as_deref(),
            Some("YouTube Music")
        );

        // Pausing the video and playing it again makes it the newest.
        let mut paused_video = video.clone();
        paused_video.paused = true;
        selector.select(
            &[paused_video, music.clone()],
            &preferences,
            base + Duration::from_secs(8),
        );
        let again = base + Duration::from_secs(10);
        assert_eq!(
            selector
                .select(&both, &preferences, again)
                .winner
                .as_deref(),
            Some("YouTube")
        );

        // A manual activity still comes first.
        let typed = candidate("Typed", Kind::Manual);
        let all = [video, music, typed];
        let result = selector.select(&all, &preferences, base + Duration::from_secs(20));
        assert_eq!(result.winner.as_deref(), Some("Typed"));
    }

    #[test]
    fn without_grace_time_paused_media_hides_at_once() {
        let mut selector = Selector::default();
        let result = selector.select(
            &[paused("Music", Kind::Music)],
            &Preferences::default(),
            Instant::now(),
        );
        assert_eq!(result.winner, None);
        assert!(!selector.has_pending());
    }

    #[test]
    fn the_user_choice_of_member_list_line_wins_over_the_presence() {
        let mut item = candidate("Video", Kind::Video);
        item.activity.status_display = Some(StatusDisplay::Name);
        let options = PresencePreferences {
            status_display: Some(StatusDisplay::Details),
            ..Default::default()
        };
        let activity = filter_activity(&item, &GlobalPreferences::default(), &options);
        assert_eq!(activity.status_display, Some(StatusDisplay::Details));
        // A choice whose line is missing is dropped like the presence's own.
        item.activity.state = None;
        let options = PresencePreferences {
            status_display: Some(StatusDisplay::State),
            ..Default::default()
        };
        let activity = filter_activity(&item, &GlobalPreferences::default(), &options);
        assert_eq!(activity.status_display, None);
    }

    #[test]
    fn the_idle_rule_needs_the_setting_and_a_known_time() {
        let minute = Duration::from_secs(60);
        assert!(!is_idle(Some(minute * 600), None));
        assert!(!is_idle(None, Some(5)));
        assert!(!is_idle(Some(minute * 5 - Duration::from_secs(1)), Some(5)));
        assert!(is_idle(Some(minute * 5), Some(5)));
        assert!(is_idle(Some(minute * 121), Some(120)));
    }

    #[test]
    fn nothing_is_selected_while_idle_and_it_comes_back_after() {
        let mut selector = Selector::default();
        let preferences = Preferences::default();
        let offered = [
            candidate("Game", Kind::Game),
            candidate("Typed", Kind::Manual),
        ];
        let now = Instant::now();
        assert!(selector.set_idle(true));
        assert!(!selector.set_idle(true));
        let result = selector.select(&offered, &preferences, now);
        assert_eq!(result.winner, None);
        assert!(result.rows.iter().all(|row| row.reason == IDLE_REASON));

        assert!(selector.set_idle(false));
        let result = selector.select(&offered, &preferences, now);
        assert_eq!(result.winner.as_deref(), Some("Typed"));
    }

    #[test]
    fn paused_publishing_selects_nothing() {
        let mut selector = Selector::default();
        let mut preferences = Preferences::default();
        preferences.global.paused = true;
        let result = selector.select(
            &[candidate("Game", Kind::Game)],
            &preferences,
            Instant::now(),
        );
        assert_eq!(result.winner, None);
        assert_eq!(result.rows[0].reason, "Publishing is paused");
    }

    #[test]
    fn hostname_drops_everything_but_the_host() {
        assert_eq!(
            hostname("https://User:pw@WWW.Example.com:8080/a?b=1"),
            "www.example.com"
        );
        assert_eq!(hostname("not a url"), "");
    }

    #[test]
    fn quiet_hours_can_cross_midnight() {
        let night = QuietHours {
            from: 22 * 60,
            to: 7 * 60,
        };
        assert!(night.covers(23 * 60));
        assert!(night.covers(0));
        assert!(night.covers(7 * 60 - 1));
        assert!(!night.covers(7 * 60));
        assert!(!night.covers(12 * 60));
        assert!(night.covers(22 * 60));

        let day = QuietHours {
            from: 9 * 60,
            to: 17 * 60,
        };
        assert!(day.covers(9 * 60));
        assert!(!day.covers(17 * 60));
        assert!(!QuietHours { from: 60, to: 60 }.covers(60));
    }

    #[test]
    fn a_listed_program_hides_however_it_is_written() {
        let global = GlobalPreferences {
            quiet_programs: vec!["obs64".into(), "zoom.exe".into()],
            ..GlobalPreferences::default()
        };
        assert!(is_quiet(&global, &["OBS64.EXE".into()], None));
        assert!(is_quiet(
            &global,
            &["explorer.exe".into(), "Zoom".into()],
            None
        ));
        assert!(!is_quiet(&global, &["obs.exe".into()], None));
        assert!(!is_quiet(
            &GlobalPreferences::default(),
            &["obs64.exe".into()],
            Some(0)
        ));
    }

    #[test]
    fn an_unknown_clock_leaves_the_hours_out() {
        let global = GlobalPreferences {
            quiet_hours: Some(QuietHours { from: 0, to: 1439 }),
            ..GlobalPreferences::default()
        };
        assert!(is_quiet(&global, &[], Some(600)));
        assert!(!is_quiet(&global, &[], None));
    }

    #[test]
    fn a_quiet_rule_hides_a_custom_status_too() {
        let mut selector = Selector::default();
        let now = Instant::now();
        let candidates = [candidate("manual", Kind::Manual)];
        let preferences = Preferences::default();
        assert!(
            selector
                .select(&candidates, &preferences, now)
                .winner
                .is_some()
        );

        assert!(selector.set_quiet(true));
        assert!(!selector.set_quiet(true));
        let hidden = selector.select(&candidates, &preferences, now);
        assert!(hidden.winner.is_none());
        assert_eq!(hidden.rows[0].reason, QUIET_REASON);

        assert!(selector.set_quiet(false));
        assert!(
            selector
                .select(&candidates, &preferences, now)
                .winner
                .is_some()
        );
    }

    /// The name is a text like any other: a script could put a page title in it.
    #[test]
    fn hiding_the_title_hides_a_name_the_script_wrote() {
        let mut video = candidate("YouTube", Kind::Video);
        video.activity.name = Some("A private video".into());
        let no_titles = PresencePreferences {
            show_title: false,
            ..PresencePreferences::default()
        };
        let filtered = filter_activity(&video, &GlobalPreferences::default(), &no_titles);
        assert_eq!(filtered.name.as_deref(), Some("YouTube"));

        let service = GlobalPreferences {
            privacy: PrivacyMode::Service,
            ..GlobalPreferences::default()
        };
        let filtered = filter_activity(&video, &service, &PresencePreferences::default());
        assert_eq!(filtered.name.as_deref(), Some("YouTube"));

        let shown = filter_activity(
            &video,
            &GlobalPreferences::default(),
            &PresencePreferences::default(),
        );
        assert_eq!(shown.name.as_deref(), Some("A private video"));
    }
}

#[cfg(test)]
mod random_input {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn an_exclusion_holds_however_the_domain_is_written(
            sub in "([a-z]{1,8}\\.)?",
            name in "[a-z]{1,10}",
            port in proptest::option::of(1_u16..),
            www in any::<bool>(),
            dot in any::<bool>(),
            upper in any::<bool>(),
        ) {
            let excluded = format!("{name}.test");
            let mut written = format!("{sub}{excluded}");
            if www {
                written.insert_str(0, "www.");
            }
            if dot {
                written.push('.');
            }
            if let Some(port) = port {
                written.push_str(&format!(":{port}"));
            }
            if upper {
                written = written.to_uppercase();
            }
            prop_assert!(domain_excluded(&written, &[excluded]), "{} got through", written);
        }

        #[test]
        fn a_longer_name_is_not_the_excluded_one(prefix in "[a-z]{1,8}", name in "[a-z]{1,10}") {
            let excluded = format!("{name}.test");
            let other = format!("{prefix}{excluded}");
            prop_assert!(!domain_excluded(&other, &[excluded]));
        }

        #[test]
        fn any_text_is_safe_as_a_domain(text in "\\PC{0,120}", list in "\\PC{0,40}") {
            let _ = domain_excluded(&text, &[list]);
            let _ = hostname(&text);
        }
    }
}
