//! Where the index and the presence files come from.

use std::time::Duration;

use crate::error::Error;
use crate::index::{Channel, MAX_INDEX};

/// Time allowed for one download.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Longest entity tag kept; GitHub's are 66 characters.
const MAX_TAG: usize = 128;

/// What asking for the index again gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fetched {
    /// A copy of the index, and the tag that names it when the server gave one.
    New {
        /// The signed file, still to be verified.
        bytes: Vec<u8>,
        /// Sent back next time, so an index that did not change is not downloaded again.
        tag: Option<String>,
    },
    /// The index is still the copy the tag named.
    Unchanged,
}

/// Something that serves the index and the files of a commit.
pub trait Source: Send + Sync {
    /// Downloads the signed `catalog.json` of a channel, at most [`MAX_INDEX`]
    /// bytes.
    ///
    /// Raises:
    ///     Error: The download fails or the file is larger than the limit.
    fn index(&self, channel: Channel) -> Result<Vec<u8>, Error>;

    /// Downloads the index unless it is still the copy named by `known`, a tag
    /// from an earlier [`Fetched::New`].
    ///
    /// Raises:
    ///     Error: As [`Source::index`].
    fn index_if_changed(&self, channel: Channel, known: Option<&str>) -> Result<Fetched, Error> {
        let _ = known;
        self.index(channel)
            .map(|bytes| Fetched::New { bytes, tag: None })
    }

    /// Downloads one file of a presence at an exact commit.
    ///
    /// Args:
    ///     commit: Full SHA of the commit the index was made from.
    ///     folder: Folder of the presence under `presences/`.
    ///     size: Length the index declares; a longer answer may be cut at one more byte.
    fn file(&self, commit: &str, folder: &str, path: &str, size: u64) -> Result<Vec<u8>, Error>;

    /// Downloads the bundle of a presence version from a release. The bytes
    /// still have to be checked against the index.
    ///
    /// Args:
    ///     tag: From [`Channel::release`].
    ///     name: File name the signed index gives.
    ///     limit: Most bytes read; a longer answer is cut there.
    ///
    /// Raises:
    ///     Error: The download fails. A source without bundles always fails,
    ///         and the files are then fetched one by one.
    fn bundle(&self, tag: &str, name: &str, limit: u64) -> Result<Vec<u8>, Error> {
        let _ = (tag, name, limit);
        Err(Error::Network("this source has no bundles".into()))
    }
}

/// Hosts a GitHub release download may redirect to. GitHub serves release
/// files from these; any other place is refused.
const RELEASE_HOSTS: [&str; 2] = [
    "release-assets.githubusercontent.com",
    "objects.githubusercontent.com",
];

/// Whether a redirect from a release download goes to GitHub's file hosts
/// over HTTPS, on the default port and without credentials in the address.
fn release_host(location: &str) -> bool {
    let Some(rest) = location.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    RELEASE_HOSTS.contains(&authority)
}

/// Serves the catalog from a public GitHub repository.
#[derive(Debug, Clone)]
pub struct Github {
    owner: String,
    repo: String,
}

fn plain(name: &str) -> bool {
    // A name made only of dots would be a path step ("." or "..") in the address.
    !name.is_empty()
        && name.len() <= 100
        && !name.bytes().all(|b| b == b'.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

impl Github {
    /// Points at a repository.
    ///
    /// Raises:
    ///     Error: Either name could change the address it is placed in.
    pub fn new(owner: &str, repo: &str) -> Result<Self, Error> {
        if !plain(owner) || !plain(repo) {
            return Err(Error::Invalid("the repository name is not plain".into()));
        }
        Ok(Self {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
        })
    }

    fn index_url(&self, channel: Channel) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/catalog.json",
            self.owner,
            self.repo,
            channel.branch()
        )
    }

    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, Error> {
        // Redirects are refused: raw.githubusercontent.com answers directly,
        // and a redirect would send a signed download somewhere unvetted.
        let response = call(url, None)?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(refused(status, url));
        }
        read(response, limit)
    }
}

/// The error for an answer other than 200.
fn refused(status: u16, url: &str) -> Error {
    if status == 429 {
        Error::RateLimited
    } else {
        Error::Network(format!("{status} for {url}"))
    }
}

/// A tag worth sending back: short, quoted visible ASCII, as HTTP writes it.
fn usable_tag(tag: &str) -> bool {
    tag.len() <= MAX_TAG
        && tag.len() >= 2
        && tag.bytes().all(|b| b.is_ascii_graphic())
        && tag.trim_start_matches("W/").starts_with('"')
        && tag.ends_with('"')
}

fn call(url: &str, known: Option<&str>) -> Result<ureq::http::Response<ureq::Body>, Error> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into();
    let mut request = agent.get(url);
    if let Some(tag) = known.filter(|tag| usable_tag(tag)) {
        request = request.header("If-None-Match", tag);
    }
    request.call().map_err(|e| Error::Network(e.to_string()))
}

fn read(mut response: ureq::http::Response<ureq::Body>, limit: u64) -> Result<Vec<u8>, Error> {
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|e| Error::Network(e.to_string()))
}

impl Source for Github {
    fn index(&self, channel: Channel) -> Result<Vec<u8>, Error> {
        self.get(&self.index_url(channel), MAX_INDEX)
    }

    fn index_if_changed(&self, channel: Channel, known: Option<&str>) -> Result<Fetched, Error> {
        let url = self.index_url(channel);
        let response = call(&url, known)?;
        let status = response.status().as_u16();
        // A 304 only counts as an answer to a tag that was sent.
        if status == 304 && known.is_some_and(usable_tag) {
            return Ok(Fetched::Unchanged);
        }
        if status != 200 {
            return Err(refused(status, &url));
        }
        let tag = response
            .headers()
            .get("etag")
            .and_then(|value| value.to_str().ok())
            .filter(|tag| usable_tag(tag))
            .map(str::to_owned);
        Ok(Fetched::New {
            bytes: read(response, MAX_INDEX)?,
            tag,
        })
    }

    fn file(&self, commit: &str, folder: &str, path: &str, size: u64) -> Result<Vec<u8>, Error> {
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{commit}/presences/{folder}/{path}",
            self.owner, self.repo
        );
        self.get(&url, size.saturating_add(1))
    }

    fn bundle(&self, tag: &str, name: &str, limit: u64) -> Result<Vec<u8>, Error> {
        if !plain(name) || !plain(tag) {
            return Err(Error::Invalid("the bundle name is not plain".into()));
        }
        // github.com answers a release download with one redirect to its file
        // host. That redirect is followed by hand, once, and only to GitHub.
        let url = format!(
            "https://github.com/{}/{}/releases/download/{tag}/{name}",
            self.owner, self.repo
        );
        let first = call(&url, None)?;
        let status = first.status().as_u16();
        if status == 200 {
            return read(first, limit);
        }
        if !matches!(status, 301 | 302 | 303 | 307 | 308) {
            return Err(refused(status, &url));
        }
        let location = first
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .filter(|location| release_host(location))
            .ok_or_else(|| Error::Network(format!("{url} redirected outside GitHub")))?
            .to_owned();
        let second = call(&location, None)?;
        let status = second.status().as_u16();
        if status != 200 {
            return Err(refused(status, "a release file"));
        }
        read(second, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_names_cannot_change_the_address() {
        assert!(Github::new("manucabral", "RichPresencePlus").is_ok());
        assert!(Github::new("some.org", "my_repo-2.0").is_ok());
        for bad in [
            "",
            "a/b",
            "a b",
            "a?b",
            ".",
            "..",
            "a#b",
            "a@b",
            &"a".repeat(101),
        ] {
            assert!(Github::new(bad, "repo").is_err(), "{bad:?}");
            assert!(Github::new("owner", bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn only_a_short_quoted_tag_is_sent_back() {
        assert!(usable_tag(
            "\"acca9652162dbecb151fc9853567cc23e329a4ec12c31b9ddb1231af4a266009\""
        ));
        assert!(usable_tag("W/\"abc\""));
        for bad in [
            "",
            "\"",
            "abc",
            "\"a b\"",
            "\"a\r\nX-Evil: 1\"",
            "\"é\"",
            &format!("\"{}\"", "a".repeat(MAX_TAG)),
        ] {
            assert!(!usable_tag(bad), "{bad:?}");
        }
    }

    #[test]
    fn release_downloads_only_follow_github_file_hosts() {
        assert!(release_host(
            "https://release-assets.githubusercontent.com/github-production-release-asset/1/2?sp=r&sig=x"
        ));
        assert!(release_host("https://objects.githubusercontent.com/x"));
        for bad in [
            "http://release-assets.githubusercontent.com/x",
            "https://release-assets.githubusercontent.com.evil.com/x",
            "https://evil.com/release-assets.githubusercontent.com",
            "https://user@release-assets.githubusercontent.com/x",
            "https://release-assets.githubusercontent.com:8443/x",
            "https://example.com/?https://objects.githubusercontent.com",
            "//objects.githubusercontent.com/x",
            "",
        ] {
            assert!(!release_host(bad), "{bad}");
        }
    }
}
