//! Local WebSocket server for the browser extension.
//!
//! Loopback only, extension origins only, and the token never travels.

mod changes;
pub mod native;
pub mod protocol;
pub mod secure;
mod server;
pub mod sessions;

pub use protocol::{Browser, Page, Rule, SiteRequest, VERSION};
pub use server::{Bridge, Config, generate_token};
pub use sessions::{EXPECTED_EXTENSION, SessionView};
