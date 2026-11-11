//! The public keys this build trusts. The private keys never go in the repository.

/// Keys whose signatures are accepted, by identifier. Several can be listed so
/// a new key can be introduced before the old one is retired.
pub const TRUSTED: &[(&str, &str)] = &[("main", "nTtJ5MYzwpQuf//kwTGB7QvB5I7VT+bQtjbjYGStTuU=")];

/// Owner of the GitHub repositories the catalog is read from.
pub const REPOSITORY_OWNER: &str = "manucabral";

/// Name of the GitHub repository where presences are reviewed; the index
/// names a commit of it.
pub const REPOSITORY_NAME: &str = "RichPresencePlus";

/// Name of the GitHub repository that holds what is published: the signed
/// index, the bundles and the install counts.
pub const CATALOG_REPOSITORY_NAME: &str = "RichPresencePlus-catalog";

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
