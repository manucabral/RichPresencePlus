//! What plugin runners and the application send to the publisher.

use rpp_core::{Candidate, Preferences};

/// A message for the publisher task.
#[derive(Debug)]
pub enum Message {
    /// A source offers or refreshes its activity.
    Candidate(Box<Candidate>),
    /// A source has nothing to show.
    Clear {
        /// Identifier of the source.
        source: String,
    },
    /// A source failed; its activity is withdrawn until it recovers.
    Failed {
        /// Identifier of the source.
        source: String,
        /// What went wrong, for the user.
        error: String,
    },
    /// A presence stopped or started finding anything on its open pages.
    Health {
        /// Identifier of the source.
        source: String,
        /// True while its pages give it nothing to read.
        blank: bool,
    },
    /// The user changed the publishing preferences.
    Preferences(Box<Preferences>),
    /// The computer became idle (`true`) or the user is back (`false`).
    Idle(bool),
    /// A rule of the person began to hide everything (`true`) or stopped.
    Quiet(bool),
    /// Clear the activity and connect again, for example to another Discord client.
    Reconnect,
    /// Remove the activity and end the task.
    Stop,
}
