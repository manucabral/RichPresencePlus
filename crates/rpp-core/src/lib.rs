//! Pure rules of Rich Presence Plus: what an activity is, which one is shown
//! and what is hidden. Nothing here touches the network, disk or clock.

pub mod activity;
pub mod backoff;
pub mod local_api;
pub mod locale;
pub mod policy;
pub mod quiet;
pub mod usage;
pub mod validation;

pub use activity::{Activity, ActivityError, ActivityKind, Button, StatusDisplay};
pub use backoff::Backoff;
pub use policy::{Candidate, Kind, Preferences, Selection, Selector};
pub use quiet::Quiet;
pub use validation::PreferenceError;
