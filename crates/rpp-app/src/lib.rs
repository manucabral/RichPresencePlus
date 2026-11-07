//! The application behind the window.

mod cases;
mod catalog;
mod contribution;
mod controller;
mod develop;
pub mod diagnostics;
mod editor;
mod files;
mod folders;
mod history;
mod library;
mod local_api;
mod manifest_edit;
mod manual;
mod profiles;
mod program;
mod running;
mod site;
mod templates;
mod usage;
mod versions;
pub mod views;
mod watcher;
mod workbench;

pub use cases::Case;
pub use controller::{Controller, Options, StartError};
pub use editor::visual_studio_code;
pub use manifest_edit::ManifestChanges;
pub use program::ProgramPresence;
pub use rpp_config::Origin;
pub use templates::Template;
