//! Presence scripts: the manifest, the permissions it asks for and the
//! sandboxed JavaScript engine that runs the script.
//!
//! A script defines two global functions:
//!
//! ```js
//! function read(context) { /* gather data; return plain JSON */ }
//! function activity(snapshot) { /* return an activity object or null */ }
//! ```

mod console;
pub mod engine;
pub mod manifest;
pub mod names;
pub mod page;
pub mod permissions;
pub mod setting;
pub mod tags;

pub use engine::{Meta, Plugin, PluginError, Tick};
pub use manifest::{FORMAT, Manifest, ManifestError};
pub use names::{MAX_NAME, identity, reserved_device, valid_name, valid_person};
pub use page::{Extractor, FrameSpec, PageSpec, StorageArea};
pub use permissions::{Denied, Grant, Host, Permissions, Request};
pub use setting::{Setting, When, resolve as resolve_settings};
pub use tags::{TAGS, is_known_tag};
