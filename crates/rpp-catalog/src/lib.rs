//! The catalog of approved presences.

pub mod bundle;
pub mod catalog;
pub mod error;
pub mod freshness;
pub mod history;
pub mod index;
pub mod install;
pub mod keys;
pub mod seal;
pub mod source;

pub use catalog::{Catalog, Listing, State};
pub use error::Error;
pub use index::{Channel, Entry, FileInfo, Index, Keys};
pub use install::{Installed, Standing};
pub use seal::Key;
pub use source::{Github, Source};
