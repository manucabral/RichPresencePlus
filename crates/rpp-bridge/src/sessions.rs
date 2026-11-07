//! Which browser extensions are connected, and the last ones that were.

use std::collections::{BTreeMap, VecDeque};

use serde::Serialize;

use crate::protocol::Browser;

/// Ended sessions kept for the Browser page.
pub const RECENT_SESSIONS: usize = 10;
/// Longest version kept; real ones look like `24.12.1.712`.
const MAX_VERSION: usize = 24;

/// One connection of a browser extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionView {
    /// The browser the extension runs in.
    pub browser: Browser,
    /// Its version as the extension read it, or empty when unknown.
    pub version: String,
    /// When the extension proved itself, in Unix seconds.
    pub connected_at: u64,
    /// When the connection closed, in Unix seconds; `None` while it is open.
    pub ended_at: Option<u64>,
    /// Tabs with a page reported for a presence: now, or when it ended.
    pub tabs: usize,
}

/// Keeps a version made of digits and dots and empties anything else, so
/// nothing else from the other side of the socket reaches the interface or
/// the log.
pub fn clean_version(version: &str) -> String {
    let plain = !version.is_empty()
        && version.len() <= MAX_VERSION
        && version.bytes().all(|b| b.is_ascii_digit() || b == b'.');
    if plain {
        version.to_owned()
    } else {
        String::new()
    }
}

/// Open sessions by connection, and the last ones that ended.
#[derive(Debug, Default)]
pub(crate) struct Sessions {
    live: BTreeMap<u64, SessionView>,
    recent: VecDeque<SessionView>,
}

impl Sessions {
    /// Records a connection that completed the hello.
    pub(crate) fn open(&mut self, id: u64, browser: Browser, version: &str, now: u64) {
        self.live.insert(
            id,
            SessionView {
                browser,
                version: clean_version(version),
                connected_at: now,
                ended_at: None,
                tabs: 0,
            },
        );
    }

    /// Moves a connection to the recent list; the oldest falls off past the limit.
    pub(crate) fn close(&mut self, id: u64, now: u64) {
        let Some(mut session) = self.live.remove(&id) else {
            return;
        };
        session.ended_at = Some(now);
        self.recent.push_front(session);
        self.recent.truncate(RECENT_SESSIONS);
    }

    /// Sets how many tabs each open connection reports; one left out has none.
    pub(crate) fn set_tabs(&mut self, counts: &BTreeMap<u64, usize>) {
        for (id, session) in &mut self.live {
            session.tabs = counts.get(id).copied().unwrap_or(0);
        }
    }

    /// How many connections are open.
    pub(crate) fn count(&self) -> usize {
        self.live.len()
    }

    /// The browser of each open connection, sorted.
    pub(crate) fn browsers(&self) -> Vec<Browser> {
        let mut browsers: Vec<Browser> = self.live.values().map(|s| s.browser).collect();
        browsers.sort();
        browsers
    }

    /// Open sessions, oldest first.
    pub(crate) fn live(&self) -> Vec<SessionView> {
        let mut live: Vec<SessionView> = self.live.values().cloned().collect();
        live.sort_by_key(|session| session.connected_at);
        live
    }

    /// Ended sessions, newest first.
    pub(crate) fn recent(&self) -> Vec<SessionView> {
        self.recent.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_keep_only_digits_and_dots() {
        assert_eq!(clean_version("24.12.1.712"), "24.12.1.712");
        for bad in ["", "1.0<script>", "1 2", "9".repeat(25).as_str(), "١٢"] {
            assert_eq!(clean_version(bad), "", "{bad}");
        }
    }

    #[test]
    fn a_closed_session_moves_to_the_recent_list_with_its_end() {
        let mut sessions = Sessions::default();
        sessions.open(1, Browser::Yandex, "24.12", 100);
        sessions.open(2, Browser::Firefox, "x", 110);
        sessions.set_tabs(&BTreeMap::from([(1, 3)]));
        assert_eq!(sessions.count(), 2);
        assert_eq!(sessions.browsers(), [Browser::Firefox, Browser::Yandex]);
        let live = sessions.live();
        assert_eq!(live[0].tabs, 3);
        assert_eq!(live[1].version, "");

        sessions.close(1, 160);
        sessions.close(1, 170); // A second close changes nothing.
        assert_eq!(sessions.count(), 1);
        let recent = sessions.recent();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].browser, Browser::Yandex);
        assert_eq!(recent[0].ended_at, Some(160));
        assert_eq!(recent[0].tabs, 3);
    }

    #[test]
    fn only_the_last_sessions_are_kept() {
        let mut sessions = Sessions::default();
        for id in 0..(RECENT_SESSIONS as u64 + 5) {
            sessions.open(id, Browser::Chrome, "1", id);
            sessions.close(id, id + 1);
        }
        let recent = sessions.recent();
        assert_eq!(recent.len(), RECENT_SESSIONS);
        // Newest first.
        assert_eq!(recent[0].connected_at, RECENT_SESSIONS as u64 + 4);
        assert_eq!(sessions.count(), 0);
    }
}
