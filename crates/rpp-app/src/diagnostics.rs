//! The diagnostics page and the report a user can copy when asking for help.
//!
//! The report leaves out addresses, titles, activities, the Discord user name
//! and the text of presence errors.

use std::fmt::Write as _;

use rpp_bridge::{Browser, SessionView};
use rpp_config::{Problem, Settings};
use serde::Serialize;

use crate::Controller;
use crate::controller::lock;

/// Memory and processor time of a group of processes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Resources {
    /// Working set, in bytes.
    pub memory_bytes: u64,
    /// Processor time since the processes started, in seconds.
    pub cpu_seconds: f64,
    /// How many processes were measured.
    pub processes: usize,
}

/// The application and its WebView, measured by the shell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Measured {
    /// This process.
    pub app: Resources,
    /// The processes that draw the window.
    pub webview: Resources,
}

/// How one presence is doing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PresenceHealth {
    /// Presence name.
    pub name: String,
    /// Version from its manifest.
    pub version: String,
    /// The user has it turned on.
    pub enabled: bool,
    /// Its script is running.
    pub running: bool,
    /// Its last error, for the page only; the report says only that there is one.
    pub error: Option<String>,
}

/// Everything the diagnostics page shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diagnostics {
    /// Application version.
    pub version: String,
    /// Memory and processor time; `None` when they could not be read.
    pub resources: Option<Measured>,
    /// A Discord client is running.
    pub discord_running: bool,
    /// Discord answered a handshake.
    pub discord_connected: bool,
    /// Last problem talking to Discord.
    pub delivery_error: Option<String>,
    /// Publishing is paused.
    pub paused: bool,
    /// Browser extensions connected now.
    pub extensions: Vec<SessionView>,
    /// Every presence found.
    pub presences: Vec<PresenceHealth>,
    /// Presence folders that could not be loaded.
    pub problems: Vec<Problem>,
    /// The text to copy, built by [`Diagnostics::report`].
    pub report: String,
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
}

fn browser_name(browser: Browser) -> &'static str {
    match browser {
        Browser::Chrome => "Chrome",
        Browser::Edge => "Edge",
        Browser::Firefox => "Firefox",
        Browser::Opera => "Opera",
        Browser::Brave => "Brave",
        Browser::Vivaldi => "Vivaldi",
        Browser::Yandex => "Yandex",
        Browser::Other => "other",
    }
}

/// Replaces the user's folder in a text, so paths do not carry the account name.
fn without_home(text: &str, home: Option<&str>) -> String {
    match home {
        Some(home) if !home.is_empty() => text.replace(home, "%USERPROFILE%"),
        _ => text.to_owned(),
    }
}

impl Diagnostics {
    /// The text a user can paste when asking for help: no addresses, titles,
    /// activities, Discord user name or text of presence errors, and the
    /// user's folder `home` replaced wherever it appears.
    pub fn report(&self, home: Option<&str>) -> String {
        let yes_no = |value: bool| if value { "yes" } else { "no" };
        let mut out = String::new();
        let _ = writeln!(out, "Rich Presence Plus {}", self.version);
        let _ = writeln!(
            out,
            "System: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        match &self.resources {
            Some(measured) => {
                let _ = writeln!(
                    out,
                    "App: {}, {:.1} s CPU; WebView: {} in {} processes, {:.1} s CPU",
                    megabytes(measured.app.memory_bytes),
                    measured.app.cpu_seconds,
                    megabytes(measured.webview.memory_bytes),
                    measured.webview.processes,
                    measured.webview.cpu_seconds,
                );
            }
            None => {
                let _ = writeln!(out, "Resources: not available");
            }
        }
        let _ = writeln!(
            out,
            "Discord: running {}, connected {}, paused {}",
            yes_no(self.discord_running),
            yes_no(self.discord_connected),
            yes_no(self.paused),
        );
        if let Some(error) = &self.delivery_error {
            let _ = writeln!(out, "Discord error: {}", without_home(error, home));
        }
        let browsers: Vec<String> = self
            .extensions
            .iter()
            .map(|session| {
                let version = if session.version.is_empty() {
                    "?"
                } else {
                    &session.version
                };
                format!("{} {version}", browser_name(session.browser))
            })
            .collect();
        let _ = writeln!(
            out,
            "Extensions: {}",
            if browsers.is_empty() {
                "none".to_owned()
            } else {
                browsers.join(", ")
            }
        );
        let _ = writeln!(out, "Presences:");
        for presence in &self.presences {
            let state = match (presence.enabled, presence.running, presence.error.is_some()) {
                (false, _, _) => "off",
                (true, _, true) => "error",
                (true, true, false) => "running",
                (true, false, false) => "stopped",
            };
            let _ = writeln!(out, "  {} {}: {state}", presence.name, presence.version);
        }
        for problem in &self.problems {
            let _ = writeln!(
                out,
                "Not loaded: {}: {}",
                without_home(&problem.folder, home),
                without_home(&problem.message, home)
            );
        }
        out
    }
}

impl<H> Controller<H>
where
    H: rpp_plugins::Host + Clone + Send + 'static,
{
    /// The state the diagnostics page shows and the report built from it,
    /// read on demand only.
    ///
    /// Args:
    ///     resources: Memory and processor time measured by the shell, if any.
    pub fn diagnostics(&self, resources: Option<Measured>, home: Option<&str>) -> Diagnostics {
        let status = self.service.status();
        let running = self.service.running();
        let presences = {
            let settings: &Settings = &lock(&self.settings);
            lock(&self.library)
                .presences
                .iter()
                .map(|presence| {
                    let name = presence.manifest.name.clone();
                    PresenceHealth {
                        version: presence.manifest.version.clone(),
                        enabled: !settings.disabled.contains(&name),
                        running: running.contains(&name),
                        error: status.source_errors.get(&name).cloned(),
                        name,
                    }
                })
                .collect()
        };
        let mut diagnostics = Diagnostics {
            version: self.version.clone(),
            resources,
            discord_running: self.discord_running(),
            discord_connected: status.discord_user.is_some(),
            delivery_error: status.delivery_error.clone(),
            paused: lock(&self.settings).preferences.global.paused,
            extensions: self.bridge.live_sessions(),
            presences,
            problems: lock(&self.library).problems.clone(),
            report: String::new(),
        };
        diagnostics.report = diagnostics.report(home);
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Diagnostics {
        Diagnostics {
            version: "1.2.3".into(),
            resources: Some(Measured {
                app: Resources {
                    memory_bytes: 50 * 1024 * 1024,
                    cpu_seconds: 3.0,
                    processes: 1,
                },
                webview: Resources {
                    memory_bytes: 200 * 1024 * 1024,
                    cpu_seconds: 9.5,
                    processes: 4,
                },
            }),
            discord_running: true,
            discord_connected: true,
            delivery_error: None,
            paused: false,
            extensions: vec![SessionView {
                browser: Browser::Yandex,
                version: "24.12".into(),
                connected_at: 1,
                ended_at: None,
                tabs: 2,
            }],
            presences: vec![
                PresenceHealth {
                    name: "YouTube".into(),
                    version: "2.0.0".into(),
                    enabled: true,
                    running: true,
                    error: Some("could not read 'My Secret Video'".into()),
                },
                PresenceHealth {
                    name: "Roblox".into(),
                    version: "2.1.0".into(),
                    enabled: false,
                    running: false,
                    error: None,
                },
            ],
            problems: vec![Problem {
                folder: r"C:\Users\ana\AppData\Local\rpp\presences\broken".into(),
                message: "manifest is not valid".into(),
            }],
            report: String::new(),
        }
    }

    #[test]
    fn the_report_names_states_without_private_text() {
        let report = sample().report(Some(r"C:\Users\ana"));
        assert!(report.contains("Rich Presence Plus 1.2.3"));
        assert!(report.contains("App: 50.0 MB, 3.0 s CPU; WebView: 200.0 MB in 4 processes"));
        assert!(report.contains("Extensions: Yandex 24.12"));
        assert!(report.contains("YouTube 2.0.0: error"));
        assert!(report.contains("Roblox 2.1.0: off"));
        assert!(!report.contains("Secret"), "{report}");
        assert!(!report.contains("ana"), "{report}");
        assert!(report.contains(r"%USERPROFILE%\AppData"));
    }

    #[test]
    fn missing_figures_are_said_plainly() {
        let mut empty = sample();
        empty.resources = None;
        empty.extensions.clear();
        let report = empty.report(None);
        assert!(report.contains("Resources: not available"));
        assert!(report.contains("Extensions: none"));
    }
}
