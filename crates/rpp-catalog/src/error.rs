//! Why a catalog operation failed.

use rpp_plugins::Grant;

/// A failure while reading the catalog or installing from it.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The application carries no trusted key, so nothing can be verified.
    #[error("catalog signing keys are not configured; installing is disabled")]
    Disabled,
    /// The index or a file could not be downloaded.
    #[error("cannot reach the catalog: {0}")]
    Network(String),
    /// GitHub refused for a while because this network asked too often. Many
    /// people behind one address (a school, a shared mobile network) share
    /// the limit.
    #[error("GitHub is limiting downloads from this network for a while; try again later")]
    RateLimited,
    /// The signature is missing, wrong or made by a key that is not trusted.
    #[error("the catalog signature is missing, invalid or not trusted")]
    Signature,
    /// The signed index has a shape the application refuses.
    #[error("the catalog is not valid: {0}")]
    Invalid(String),
    /// The index is older than one already accepted for this channel.
    #[error("the catalog is older than one already accepted")]
    Stale,
    /// The name is not in the signed index.
    #[error("{0} is not in the catalog")]
    Unknown(String),
    /// A downloaded file differs from what the signed index lists.
    #[error("{0} does not match the signed catalog")]
    Integrity(String),
    /// The presence was not installed from the catalog, so the catalog does not manage it.
    #[error("{0} was not installed from the catalog")]
    NotManaged(String),
    /// The update reaches for what the installed copy does not, and the person
    /// has not accepted it. The installed copy stays as it was.
    #[error("this update asks for more access")]
    NewAccess(Vec<Grant>),
    /// A file or folder could not be written, moved or removed.
    #[error("cannot update the presences folder: {0}")]
    Disk(String),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Disk(error.to_string())
    }
}
