//! The signed index of approved presences.

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, VerifyingKey};
use rpp_plugins::reserved_device;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Highest `schema_version` this application understands.
pub const SCHEMA_VERSION: u64 = 1;
/// Largest index accepted, in bytes.
pub const MAX_INDEX: u64 = 8 * 1024 * 1024;
/// Most presences in one index.
const MAX_PRESENCES: usize = 4096;
/// Most files in one presence.
const MAX_FILES: usize = 128;
/// Largest file of a presence, in bytes.
pub const MAX_FILE: u64 = 1024 * 1024;
/// Largest presence, in bytes.
const MAX_TOTAL: u64 = 8 * 1024 * 1024;
/// Deepest folder nesting inside a presence.
const MAX_DEPTH: usize = 4;

/// A release channel. Each one has its own branch and its own index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// Reviewed releases only.
    Stable,
    /// Pre-releases for people who want to test early.
    Experimental,
}

impl Channel {
    /// Name of the branch that holds this channel's `catalog.json`.
    pub fn branch(self) -> &'static str {
        match self {
            Channel::Stable => "catalog",
            Channel::Experimental => "catalog-experimental",
        }
    }

    /// Tag of the GitHub release that holds the channel's bundles. One release
    /// holds 1000 files, about 500 presences with two versions each.
    pub fn release(self) -> &'static str {
        match self {
            Channel::Stable => "catalog-files",
            Channel::Experimental => "catalog-files-experimental",
        }
    }
}

/// Size and digest of one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileInfo {
    /// Lower-case hexadecimal SHA-256 of the contents.
    pub sha256: String,
    /// Length in bytes.
    pub size: u64,
}

/// One approved presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// Name shown to the user and used by the manifest.
    pub name: String,
    /// Folder under `presences/` in the repository, and on disk after install.
    pub folder: String,
    /// Version of the presence.
    pub version: Version,
    /// What it shows, in English.
    pub description: Option<String>,
    /// What it shows in other languages, by language tag.
    pub descriptions: BTreeMap<String, String>,
    /// Other names it is searched by.
    pub other_names: Vec<String>,
    /// Group: `game`, `video`, `music` and so on.
    pub kind: Option<String>,
    /// Every file of the presence, by path inside its folder.
    pub files: BTreeMap<String, FileInfo>,
    /// Who wrote it.
    pub author: Option<String>,
    /// Color of its card, as `#RRGGBB`.
    pub color: Option<String>,
    /// Words that describe it.
    pub tags: Vec<String>,
    /// Its logo, as an `https://` address.
    pub icon: Option<String>,
    /// Name of the release file with all its files (see [`crate::bundle`]),
    /// when the catalog published one.
    pub bundle: Option<String>,
    /// Manifest format it is written for; see [`rpp_plugins::FORMAT`].
    pub format: u32,
}

impl Entry {
    /// Tells whether it needs a newer application than this one.
    pub fn too_new(&self) -> bool {
        self.format > rpp_plugins::FORMAT
    }
}

/// A verified index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    /// Channel it belongs to.
    pub channel: Channel,
    /// Commit the files must be downloaded from.
    pub commit: String,
    /// When the maintainer generated it, in Unix seconds.
    pub generated_at: u64,
    /// The approved presences.
    pub presences: Vec<Entry>,
}

#[derive(Deserialize)]
struct Envelope {
    key_id: String,
    payload: String,
    signature: String,
}

#[derive(Deserialize)]
struct RawEntry {
    name: String,
    folder: String,
    version: String,
    description: Option<String>,
    kind: Option<String>,
    files: BTreeMap<String, FileInfo>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    bundle: Option<String>,
    #[serde(default)]
    descriptions: BTreeMap<String, String>,
    #[serde(default)]
    other_names: Vec<String>,
    // Indexes from before the field list presences of format 1.
    #[serde(default = "first_format")]
    format: u32,
}

fn first_format() -> u32 {
    1
}

#[derive(Deserialize)]
struct RawPayload {
    schema_version: u64,
    channel: Channel,
    commit: String,
    generated_at: u64,
    presences: Vec<RawEntry>,
}

/// The public keys the application trusts, by identifier.
#[derive(Debug, Clone, Default)]
pub struct Keys(BTreeMap<String, VerifyingKey>);

impl Keys {
    /// Reads keys given as `(identifier, base64 of the 32-byte public key)`.
    ///
    /// Raises:
    ///     Error: A key is not valid base64 or not a valid Ed25519 point.
    pub fn parse(pairs: &[(&str, &str)]) -> Result<Self, Error> {
        let mut keys = BTreeMap::new();
        for (id, encoded) in pairs {
            let bytes: [u8; 32] = STANDARD
                .decode(encoded)
                .ok()
                .and_then(|raw| raw.try_into().ok())
                .ok_or_else(|| Error::Invalid(format!("key {id} is not 32 bytes of base64")))?;
            let key = VerifyingKey::from_bytes(&bytes)
                .map_err(|_| Error::Invalid(format!("key {id} is not a valid public key")))?;
            keys.insert((*id).to_owned(), key);
        }
        Ok(Self(keys))
    }

    /// Tells whether no key is trusted.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn hex(text: &str, length: usize) -> bool {
    text.len() == length && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A folder name that stays a single, plain path segment.
pub(crate) fn plain_folder(name: &str) -> bool {
    let mut bytes = name.bytes();
    name.len() <= 64
        && !reserved_device(name)
        && bytes
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A relative path made of plain segments, with no way out of the folder.
fn contained(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    segments.len() <= MAX_DEPTH
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && *segment != "."
                && *segment != ".."
                && !reserved_device(segment)
                && segment.len() <= 64
                && !segment.starts_with('.')
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        })
}

/// Text shown next to a presence. It does not decide what is installed, so a
/// value that cannot be shown is dropped instead of refusing the whole index.
struct About {
    author: Option<String>,
    color: Option<String>,
    tags: Vec<String>,
    icon: Option<String>,
    descriptions: BTreeMap<String, String>,
    other_names: Vec<String>,
}

/// A language tag such as `es` or `pt-BR`.
fn language_tag(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let first = parts.next().unwrap_or_default();
    (2..=3).contains(&first.len())
        && first.bytes().all(|b| b.is_ascii_lowercase())
        && parts.all(|part| {
            (2..=8).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}

fn about(raw: &RawEntry) -> About {
    let author = raw
        .author
        .clone()
        .filter(|a| a.chars().count() <= 64 && !a.chars().any(char::is_control));
    let color = raw.color.clone().filter(|c| {
        c.strip_prefix('#')
            .is_some_and(|d| d.len() == 6 && d.bytes().all(|b| b.is_ascii_hexdigit()))
    });
    let tags = raw
        .tags
        .iter()
        .filter(|t| rpp_plugins::is_known_tag(t))
        .take(8)
        .cloned()
        .collect();
    let icon = raw.icon.clone().filter(|i| {
        i.len() <= 512
            && i.strip_prefix("https://")
                .is_some_and(|rest| !rest.is_empty())
            && !i.chars().any(|c| c.is_control() || c == ' ')
    });
    let plain = |text: &String, most: usize| {
        !text.trim().is_empty()
            && text.chars().count() <= most
            && !text.chars().any(char::is_control)
    };
    let descriptions = raw
        .descriptions
        .iter()
        .filter(|(tag, text)| language_tag(tag) && plain(text, 300))
        .take(16)
        .map(|(tag, text)| (tag.clone(), text.clone()))
        .collect();
    let other_names = raw
        .other_names
        .iter()
        .filter(|name| plain(name, 64))
        .take(8)
        .cloned()
        .collect();
    About {
        author,
        color,
        tags,
        icon,
        descriptions,
        other_names,
    }
}

fn entry(raw: RawEntry, channel: Channel) -> Result<Entry, Error> {
    let bad = |what: &str| Error::Invalid(format!("{}: {what}", raw.name));
    if raw.name.trim().is_empty() || raw.name.chars().count() > 64 || raw.name.contains('\0') {
        return Err(Error::Invalid("a presence has an unusable name".into()));
    }
    if !plain_folder(&raw.folder) {
        return Err(bad("the folder name is not plain"));
    }
    let version = Version::parse(&raw.version).map_err(|_| bad("the version is not semver"))?;
    if channel == Channel::Stable && !version.pre.is_empty() {
        return Err(bad("the stable channel cannot list a pre-release"));
    }
    if raw.format == 0 {
        return Err(bad("the format is 0"));
    }
    if raw.files.is_empty() || raw.files.len() > MAX_FILES {
        return Err(bad("it lists too few or too many files"));
    }
    if !raw.files.contains_key("manifest.json") {
        return Err(bad("it has no manifest.json"));
    }
    let mut total = 0_u64;
    for (path, info) in &raw.files {
        if !contained(path) {
            return Err(bad("a file path leaves the presence folder"));
        }
        if !hex(&info.sha256, 64) || info.size > MAX_FILE {
            return Err(bad("a file has a bad digest or is too large"));
        }
        total += info.size;
    }
    if total > MAX_TOTAL {
        return Err(bad("the presence is too large"));
    }
    let About {
        author,
        color,
        tags,
        icon,
        descriptions,
        other_names,
    } = about(&raw);
    // A bundle name that is not the expected one is ignored: the files are
    // then fetched one by one, which installs the same verified presence.
    let bundle = raw
        .bundle
        .filter(|name| crate::bundle::valid_name(name, &raw.folder, &raw.version));
    Ok(Entry {
        name: raw.name,
        folder: raw.folder,
        version,
        description: raw.description,
        descriptions,
        other_names,
        kind: raw.kind,
        files: raw.files,
        author,
        color,
        tags,
        icon,
        bundle,
        format: raw.format,
    })
}

impl Index {
    /// Checks the signature and the structure of a downloaded `catalog.json`.
    ///
    /// Args:
    ///     channel: The channel requested; an index for another one is refused.
    ///
    /// Raises:
    ///     Error: `Disabled` without keys, `Signature` when it is not signed by
    ///         a trusted key, `Invalid` for a wrong shape or limit.
    pub fn verify(bytes: &[u8], keys: &Keys, channel: Channel) -> Result<Self, Error> {
        if keys.is_empty() {
            return Err(Error::Disabled);
        }
        if bytes.len() as u64 > MAX_INDEX {
            return Err(Error::Invalid("the index is too large".into()));
        }
        let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| Error::Signature)?;
        let key = keys.0.get(&envelope.key_id).ok_or(Error::Signature)?;
        let signature = STANDARD
            .decode(&envelope.signature)
            .ok()
            .and_then(|raw| Signature::from_slice(&raw).ok())
            .ok_or(Error::Signature)?;
        key.verify_strict(envelope.payload.as_bytes(), &signature)
            .map_err(|_| Error::Signature)?;

        let payload: RawPayload =
            serde_json::from_str(&envelope.payload).map_err(|e| Error::Invalid(e.to_string()))?;
        if payload.schema_version != SCHEMA_VERSION {
            return Err(Error::Invalid("unsupported schema version".into()));
        }
        if payload.channel != channel {
            return Err(Error::Invalid(
                "the index belongs to another channel".into(),
            ));
        }
        if !hex(&payload.commit, 40) {
            return Err(Error::Invalid("the commit is not a full SHA".into()));
        }
        if payload.presences.len() > MAX_PRESENCES {
            return Err(Error::Invalid("too many presences".into()));
        }
        let mut names = std::collections::HashSet::new();
        let mut folders = std::collections::HashSet::new();
        let mut presences = Vec::with_capacity(payload.presences.len());
        for raw in payload.presences {
            let item = entry(raw, channel)?;
            if !names.insert(item.name.to_lowercase()) || !folders.insert(item.folder.clone()) {
                return Err(Error::Invalid(format!("{} appears twice", item.name)));
            }
            presences.push(item);
        }
        Ok(Self {
            channel,
            commit: payload.commit,
            generated_at: payload.generated_at,
            presences,
        })
    }

    /// Finds an entry by name, ignoring case.
    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.presences
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
pub(crate) mod tests;
