//! The public keys this build trusts. The private keys never go in the repository.

/// Keys whose signatures are accepted, by identifier. Several can be listed so
/// a new key can be introduced before the old one is retired.
pub const TRUSTED: &[(&str, &str)] = &[("main", "nTtJ5MYzwpQuf//kwTGB7QvB5I7VT+bQtjbjYGStTuU=")];

/// Owner of the GitHub repository that holds the approved presences.
pub const REPOSITORY_OWNER: &str = "manucabral";

/// Name of the GitHub repository that holds the approved presences.
pub const REPOSITORY_NAME: &str = "RichPresencePlus";

#[cfg(test)]
mod tests {
    use super::TRUSTED;
    use crate::Keys;

    #[test]
    fn trusted_keys_parse() {
        assert!(!TRUSTED.is_empty());
        assert!(Keys::parse(TRUSTED).is_ok());
    }
}
