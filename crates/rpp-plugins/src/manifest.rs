//! The `manifest.json` that describes a presence.

use std::collections::BTreeSet;

use rpp_core::Kind;
use serde::Deserialize;
use serde_json::Value;

use crate::names::{valid_name, valid_person};
use crate::page::PageSpec;
use crate::permissions::{Grant, MAX_ARG_FLAGS, MAX_ARG_PROGRAMS, Permissions, QUERY_PROTOCOLS};
use crate::setting::{MAX_CONDITION_KEYS, MAX_SETTINGS, Setting, When};
use crate::tags::is_known_tag;

/// The newest manifest and script format this application reads.
///
/// Raise it when a presence can ask for something an older application does not
/// have: a field, an extractor, a helper, a permission. A presence that uses it
/// declares the new number, so an older application refuses it with
/// [`ManifestError::Newer`] instead of failing halfway. A change that would break
/// presences already written needs a translation in [`Manifest::parse`] for the
/// formats before it.
pub const FORMAT: u32 = 1;

/// The longest description, in characters; the catalog holds the same limit for each language.
pub const MAX_DESCRIPTION: usize = 300;

/// Why a manifest was refused.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The document is not valid JSON or has an unknown or mistyped field.
    #[error("manifest is not valid: {0}")]
    Format(#[from] serde_json::Error),
    /// The presence is written for a newer format than [`FORMAT`].
    #[error(
        "needs a newer version of the application: it uses format {0}, this one reads up to {FORMAT}"
    )]
    Newer(u32),
    /// A field has a value the application cannot use.
    #[error("{field}: {reason}")]
    Field {
        /// Name of the field.
        field: &'static str,
        /// What is wrong with it.
        reason: &'static str,
    },
}

fn default_format() -> u32 {
    1
}

fn default_hide_when_paused() -> bool {
    true
}

fn default_interval() -> u32 {
    15
}

fn default_entry() -> String {
    "main.js".into()
}

/// Description of a presence and the permissions it asks for.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Format the presence is written for, from 1 to [`FORMAT`]; 1 when missing.
    #[serde(default = "default_format")]
    pub format: u32,
    /// Name shown to the user; also the folder name when installed.
    pub name: String,
    /// Version of the presence, such as `1.2.0`.
    pub version: String,
    /// Short explanation of what the presence shows.
    #[serde(default)]
    pub description: Option<String>,
    /// Discord application that owns the artwork.
    pub client_id: String,
    /// Group used for the default priority.
    #[serde(default)]
    pub kind: Option<Kind>,
    /// Seconds between two reads.
    #[serde(default = "default_interval")]
    pub interval: u32,
    /// Hide the activity while `meta` reports it paused. The user can change
    /// it per presence; `false` keeps it on display without its counters.
    #[serde(default = "default_hide_when_paused")]
    pub hide_when_paused: bool,
    /// Script file, relative to the presence folder.
    #[serde(default = "default_entry")]
    pub entry: String,
    /// Hostnames involved, used by the privacy exclusions.
    #[serde(default)]
    pub domains: Vec<String>,
    /// What the script may ask the host to do.
    #[serde(default)]
    pub permissions: Permissions,
    /// Pages the browser extension reads for this presence.
    #[serde(default)]
    pub page: Option<PageSpec>,
    /// Options the user can change; the script reads them from `context.options`.
    #[serde(rename = "options", default)]
    pub settings: Vec<Setting>,
    /// Who wrote the presence.
    #[serde(default)]
    pub author: Option<String>,
    /// Other people who helped.
    #[serde(default)]
    pub contributors: Vec<String>,
    /// Color of the presence's card, as `#RRGGBB`.
    #[serde(default)]
    pub color: Option<String>,
    /// Words from [`crate::TAGS`] that describe it, used to search.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Logo: an `https://` address, or a `.png` or `.webp` file in the folder.
    #[serde(default)]
    pub icon: Option<String>,
    /// Other names people search it by, such as a short form or an old name.
    #[serde(default)]
    pub other_names: Vec<String>,
}

/// An `https://` address, or a relative `.png` or `.webp` path inside the folder.
fn icon_is_valid(icon: &str) -> bool {
    if icon.len() > 512 || icon.chars().any(|c| c.is_control() || c == ' ') {
        return false;
    }
    if let Some(rest) = icon.strip_prefix("https://") {
        return !rest.is_empty() && !rest.starts_with('/');
    }
    let lower = icon.to_ascii_lowercase();
    (lower.ends_with(".png") || lower.ends_with(".webp"))
        && !icon.starts_with(['/', '\\'])
        && !icon.contains(':')
        && icon
            .split(['/', '\\'])
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// The `format` of a manifest, read before the rest: a newer format may carry
/// fields this application would call unknown, and the person must hear that the
/// application is old, not that the presence is broken.
fn format_of(json: &str) -> Result<u32, ManifestError> {
    #[derive(Deserialize)]
    struct Head {
        #[serde(default = "default_format")]
        format: u32,
    }
    Ok(serde_json::from_str::<Head>(json)?.format)
}

fn reject(field: &'static str, reason: &'static str) -> ManifestError {
    ManifestError::Field { field, reason }
}

impl Manifest {
    /// One [`Grant`] per permission and site read, so two versions can be
    /// compared.
    pub fn grants(&self) -> BTreeSet<Grant> {
        let mut grants = self.permissions.grants();
        if let Some(page) = &self.page {
            grants.extend(page.sites().cloned().map(Grant::Site));
        }
        grants
    }

    /// Parses and validates the contents of `manifest.json`.
    ///
    /// Raises:
    ///     ManifestError: `Newer` for a format after [`FORMAT`]; otherwise the
    ///         document is not valid JSON, has an unknown field or a field holds
    ///         a value the application cannot use.
    pub fn parse(json: &str) -> Result<Self, ManifestError> {
        let format = format_of(json)?;
        if format > FORMAT {
            return Err(ManifestError::Newer(format));
        }
        let mut manifest: Self = serde_json::from_str(json)?;
        manifest.validate()?;
        // A newer application may know more tags. A word this one does not know is
        // left out, so the list can grow without stopping a presence.
        manifest.tags.retain(|tag| is_known_tag(tag));
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), ManifestError> {
        if self.format == 0 {
            return Err(reject("format", "expected a whole number from 1"));
        }
        if !valid_name(&self.name) {
            return Err(reject("name", "use letters, digits, spaces, _ - and ."));
        }
        if self.version.is_empty() || self.version.len() > 32 {
            return Err(reject("version", "expected 1 to 32 characters"));
        }
        let id_ok = (17..=20).contains(&self.client_id.len())
            && self.client_id.bytes().all(|b| b.is_ascii_digit());
        if !id_ok {
            return Err(reject(
                "client_id",
                "expected the numeric Discord application id",
            ));
        }
        if !(1..=300).contains(&self.interval) {
            return Err(reject("interval", "expected 1 to 300 seconds"));
        }
        let entry_ok = self.entry.ends_with(".js")
            && !self.entry.starts_with(['/', '\\'])
            && !self.entry.contains(':')
            && self
                .entry
                .split(['/', '\\'])
                .all(|part| !part.is_empty() && part != "." && part != "..");
        if !entry_ok {
            return Err(reject(
                "entry",
                "expected a relative .js path inside the folder",
            ));
        }
        let domains_ok = self.domains.len() <= 32
            && self.domains.iter().all(|domain| {
                !domain.is_empty()
                    && domain.len() <= 253
                    && domain.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-'
                    })
            });
        if !domains_ok {
            return Err(reject("domains", "expected up to 32 lowercase hostnames"));
        }
        self.validate_args()?;
        let query = &self.permissions.query;
        let query_ok = query.iter().all(|p| QUERY_PROTOCOLS.contains(&p.as_str()))
            && query.iter().collect::<std::collections::HashSet<_>>().len() == query.len();
        if !query_ok {
            return Err(reject(
                "permissions",
                "query: use protocols the app knows, once each: samp",
            ));
        }
        if let Some(reason) = self.page.as_ref().and_then(|page| page.validate().err()) {
            return Err(reject("page", reason));
        }
        self.validate_settings()?;
        self.validate_about()
    }

    fn validate_about(&self) -> Result<(), ManifestError> {
        if let Some(text) = &self.description
            && (text.chars().count() > MAX_DESCRIPTION || text.chars().any(char::is_control))
        {
            return Err(reject("description", "one line of up to 300 characters"));
        }
        let person = |text: &String| valid_person(text);
        if !self.author.iter().all(person) || !self.contributors.iter().all(person) {
            return Err(reject("author", "names have up to 64 characters"));
        }
        if self.contributors.len() > 10 {
            return Err(reject("contributors", "list at most 10 people"));
        }
        if let Some(color) = &self.color {
            let digits = color.strip_prefix('#').unwrap_or_default();
            if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(reject("color", "expected #RRGGBB"));
            }
        }
        let tag_ok = |tag: &String| {
            (1..=24).contains(&tag.len())
                && tag
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        };
        if self.tags.len() > 8 || !self.tags.iter().all(tag_ok) {
            return Err(reject(
                "tags",
                "up to 8 lower-case words of letters, digits and -",
            ));
        }
        if let Some(icon) = &self.icon
            && !icon_is_valid(icon)
        {
            return Err(reject(
                "icon",
                "expected an https address or a .png or .webp file in the folder",
            ));
        }
        self.validate_other_names()
    }

    fn validate_other_names(&self) -> Result<(), ManifestError> {
        let mut seen = BTreeSet::from([self.name.to_lowercase()]);
        let fine = self.other_names.len() <= 8
            && self
                .other_names
                .iter()
                .all(|other| valid_person(other) && seen.insert(other.to_lowercase()));
        if fine {
            Ok(())
        } else {
            Err(reject(
                "other_names",
                "up to 8 names of up to 64 characters, each different from the name and the others",
            ))
        }
    }

    /// Programs are plain executable names; options look like `-h`, `--port`
    /// or `/server`, so a value can never be declared as if it were an option.
    fn validate_args(&self) -> Result<(), ManifestError> {
        let program_ok = |name: &str| {
            (1..=64).contains(&name.len())
                && !name.starts_with('.')
                && !crate::names::reserved_device(name)
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' '))
        };
        let flag_ok = |flag: &str| {
            (2..=32).contains(&flag.len())
                && flag.starts_with(['-', '/'])
                && flag[1..]
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                && flag[1..].bytes().any(|b| b.is_ascii_alphanumeric())
        };
        let args = &self.permissions.args;
        let ok = args.len() <= MAX_ARG_PROGRAMS
            && args.iter().all(|(program, flags)| {
                program_ok(program)
                    && (1..=MAX_ARG_FLAGS).contains(&flags.len())
                    && flags.iter().all(|flag| flag_ok(flag))
            });
        if ok {
            Ok(())
        } else {
            Err(reject(
                "permissions",
                "args: up to 16 executable names, each with 1 to 16 options such as -h or --port",
            ))
        }
    }

    fn validate_settings(&self) -> Result<(), ManifestError> {
        if self.settings.len() > MAX_SETTINGS {
            return Err(reject("options", "declare at most 16 settings"));
        }
        let mut keys = std::collections::HashSet::new();
        for setting in &self.settings {
            setting
                .validate()
                .map_err(|reason| reject("options", reason))?;
            if !keys.insert(setting.key()) {
                return Err(reject("options", "two settings share a key"));
            }
        }
        self.validate_conditions()
    }

    /// A condition looks at other settings of the same manifest, with values each
    /// of them accepts. Chains are allowed; a loop, even through several
    /// settings, is not.
    fn validate_conditions(&self) -> Result<(), ManifestError> {
        let find = |name: &str| self.settings.iter().find(|other| other.key() == name);
        for setting in &self.settings {
            let Some(when) = setting.condition() else {
                continue;
            };
            let needs = when.requirements();
            if needs.is_empty() || needs.len() > MAX_CONDITION_KEYS {
                return Err(reject("options", "\"when\" looks at 1 to 4 settings"));
            }
            for (name, allowed) in needs {
                let Some(other) = find(name).filter(|other| other.key() != setting.key()) else {
                    return Err(reject(
                        "options",
                        "\"when\" names another setting of this presence",
                    ));
                };
                if matches!(when, When::On(_)) && !matches!(other, Setting::Boolean { .. }) {
                    return Err(reject(
                        "options",
                        "\"when\" with a single name needs a boolean setting",
                    ));
                }
                let fits = |value: &Value| !value.is_array() && other.check(value).is_ok();
                if allowed.is_empty() || allowed.len() > 16 || !allowed.iter().all(fits) {
                    return Err(reject(
                        "options",
                        "\"when\" gives values the named setting can take",
                    ));
                }
            }
        }
        // A loop: following conditions from a setting comes back to it.
        let depends = |setting: &Setting| -> Vec<String> {
            setting
                .condition()
                .map(|when| {
                    when.requirements()
                        .iter()
                        .map(|(key, _)| (*key).to_owned())
                        .collect()
                })
                .unwrap_or_default()
        };
        for setting in &self.settings {
            let mut seen = std::collections::HashSet::new();
            let mut waiting = depends(setting);
            while let Some(key) = waiting.pop() {
                if key == setting.key() {
                    return Err(reject("options", "\"when\" conditions form a loop"));
                }
                if seen.insert(key.clone())
                    && let Some(other) = find(&key)
                {
                    waiting.extend(depends(other));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const GOOD: &str = r#"{
        "name": "Video Site", "version": "1.0.0", "client_id": "1113646725408772176",
        "kind": "video", "interval": 5, "domains": ["video.example.com"],
        "permissions": {"http": ["api.example.com"], "process": true}
    }"#;

    #[test]
    fn a_complete_manifest_is_accepted() {
        let manifest = Manifest::parse(GOOD).unwrap();
        assert_eq!(manifest.entry, "main.js");
        assert_eq!(manifest.kind, Some(Kind::Video));
        assert!(manifest.permissions.process);
    }

    #[test]
    fn defaults_apply_to_optional_fields() {
        let manifest =
            Manifest::parse(r#"{"name":"A","version":"1","client_id":"12345678901234567"}"#)
                .unwrap();
        assert_eq!(manifest.interval, 15);
        assert!(manifest.permissions.http.is_empty());
    }

    #[test]
    fn a_manifest_without_format_is_format_1() {
        assert_eq!(Manifest::parse(GOOD).unwrap().format, 1);
        let declared = GOOD.replace("\"name\"", "\"format\": 1, \"name\"");
        assert_eq!(Manifest::parse(&declared).unwrap().format, 1);
    }

    #[test]
    fn a_newer_format_says_the_application_is_old_even_with_fields_it_does_not_know() {
        let newer = GOOD.replace(
            "\"name\"",
            &format!(
                "\"format\": {}, \"from_the_future\": true, \"name\"",
                FORMAT + 1
            ),
        );
        let error = Manifest::parse(&newer).unwrap_err();
        assert!(matches!(error, ManifestError::Newer(f) if f == FORMAT + 1));
        assert!(
            error
                .to_string()
                .contains("needs a newer version of the application")
        );
    }

    /// Raising [`FORMAT`] without the schema and the templates would let the editor
    /// refuse the new number and new presences start on the old one.
    #[test]
    fn the_schema_and_the_templates_know_this_format() {
        let read = |text: &str| serde_json::from_str::<serde_json::Value>(text).unwrap();
        let schema = read(include_str!("../../rpp-app/templates/manifest.schema.json"));
        assert_eq!(schema["properties"]["format"]["maximum"], FORMAT);
        for template in [
            include_str!("../../rpp-app/templates/web/manifest.json"),
            include_str!("../../rpp-app/templates/desktop/manifest.json"),
        ] {
            assert_eq!(read(template)["format"], FORMAT);
        }
    }

    #[test]
    fn format_0_or_not_a_number_is_refused() {
        let zero = GOOD.replace("\"name\"", "\"format\": 0, \"name\"");
        assert!(matches!(
            Manifest::parse(&zero),
            Err(ManifestError::Field {
                field: "format",
                ..
            })
        ));
        for wrong in ["\"1\"", "1.5", "-1"] {
            let json = GOOD.replace("\"name\"", &format!("\"format\": {wrong}, \"name\""));
            assert!(
                matches!(Manifest::parse(&json), Err(ManifestError::Format(_))),
                "{wrong}"
            );
        }
    }

    #[test]
    fn unknown_fields_are_refused() {
        let json = GOOD.replace("\"interval\": 5", "\"interval\": 5, \"surprise\": 1");
        assert!(matches!(
            Manifest::parse(&json),
            Err(ManifestError::Format(_))
        ));
    }

    #[test]
    fn invalid_values_name_the_field() {
        let cases = [
            ("\"name\": \"Video Site\"", "\"name\": \"../x\"", "name"),
            // Spaces around a name would make two presences look the same.
            (
                "\"name\": \"Video Site\"",
                "\"name\": \" Video Site\"",
                "name",
            ),
            ("\"name\": \"Video Site\"", "\"name\": \"- -\"", "name"),
            ("1113646725408772176", "abc", "client_id"),
            ("\"interval\": 5", "\"interval\": 0", "interval"),
            (
                "\"domains\": [\"video.example.com\"]",
                "\"domains\": [\"Bad Host\"]",
                "domains",
            ),
            (
                "\"process\": true",
                "\"process\": true, \"args\": {\"../gta_sa.exe\": [\"-h\"]}",
                "permissions",
            ),
            (
                "\"process\": true",
                "\"process\": true, \"args\": {\"con.exe\": [\"-h\"]}",
                "permissions",
            ),
            (
                "\"process\": true",
                "\"process\": true, \"args\": {\"gta_sa.exe\": []}",
                "permissions",
            ),
            (
                "\"process\": true",
                "\"process\": true, \"args\": {\"gta_sa.exe\": [\"127.0.0.1\"]}",
                "permissions",
            ),
            (
                "\"process\": true",
                "\"process\": true, \"args\": {\"gta_sa.exe\": [\"--\"]}",
                "permissions",
            ),
            (
                "\"process\": true",
                "\"process\": true, \"query\": [\"minecraft\"]",
                "permissions",
            ),
            (
                "\"process\": true",
                "\"process\": true, \"query\": [\"samp\", \"samp\"]",
                "permissions",
            ),
        ];
        for (from, to, field) in cases {
            let error = Manifest::parse(&GOOD.replace(from, to)).unwrap_err();
            assert!(
                matches!(error, ManifestError::Field { field: f, .. } if f == field),
                "{field}"
            );
        }
    }

    #[test]
    fn entry_must_stay_inside_the_folder() {
        for entry in ["../main.js", "/main.js", "C:/main.js", "a//b.js", "main.py"] {
            let json = GOOD.replace("\"kind\"", &format!("\"entry\": \"{entry}\", \"kind\""));
            assert!(Manifest::parse(&json).is_err(), "{entry}");
        }
        let json = GOOD.replace("\"kind\"", "\"entry\": \"src/main.js\", \"kind\"");
        assert!(Manifest::parse(&json).is_ok());
    }

    #[test]
    fn presences_hide_while_paused_unless_the_manifest_says_otherwise() {
        assert!(Manifest::parse(GOOD).unwrap().hide_when_paused);
        let kept = GOOD.replace(
            "\"kind\": \"video\"",
            "\"kind\": \"video\", \"hide_when_paused\": false",
        );
        assert!(!Manifest::parse(&kept).unwrap().hide_when_paused);
    }

    #[test]
    fn a_setting_may_depend_on_another_boolean_setting() {
        let with = |settings: &str| {
            Manifest::parse(&GOOD.replace(
                "\"kind\": \"video\"",
                &format!("\"kind\": \"video\", \"options\": {settings}"),
            ))
        };
        let good = with(
            r#"[{"type":"boolean","key":"players","default":true},
                {"type":"choice","key":"style","default":"a","choices":["a","b"],"when":"players"}]"#,
        )
        .unwrap();
        assert_eq!(
            good.settings[1].condition(),
            Some(&When::On("players".into()))
        );
        assert_eq!(good.settings[0].condition(), None);
        for bad in [
            r#"[{"type":"boolean","key":"a","default":true,"when":"a"}]"#,
            r#"[{"type":"boolean","key":"a","default":true,"when":"missing"}]"#,
            r#"[{"type":"text","key":"t","default":"x"},{"type":"boolean","key":"b","default":true,"when":"t"}]"#,
            r#"[{"type":"boolean","key":"a","default":true,"when":"b"},{"type":"boolean","key":"b","default":true,"when":"a"}]"#,
            r#"[{"type":"boolean","key":"a","default":true,"when":"c"},{"type":"boolean","key":"b","default":true,"when":"a"},
                {"type":"boolean","key":"c","default":true,"when":"b"}]"#,
        ] {
            assert!(with(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_setting_may_depend_on_the_values_of_other_settings() {
        let with = |settings: &str| {
            Manifest::parse(&GOOD.replace(
                "\"kind\": \"video\"",
                &format!("\"kind\": \"video\", \"options\": {settings}"),
            ))
        };
        let base = r#"{"type":"choice","key":"image","default":"logo","choices":["logo","cover","none"]},
            {"type":"boolean","key":"show","default":true},
            {"type":"number","key":"size","default":2,"min":1,"max":5}"#;
        let good = with(&format!(
            r#"[{base}, {{"type":"text","key":"caption","default":"","when":{{"image":["cover","logo"],"show":false,"size":3}}}}]"#
        ))
        .unwrap();
        let needs = good.settings[3].condition().unwrap().requirements();
        assert_eq!(
            needs,
            vec![
                ("image", vec![json!("cover"), json!("logo")]),
                ("show", vec![json!(false)]),
                ("size", vec![json!(3)]),
            ]
        );
        for bad in [
            // A value the named setting cannot take.
            r#"{"image":"poster"}"#,
            r#"{"size":9}"#,
            r#"{"show":"yes"}"#,
            r#"{"image":[["cover"]]}"#,
            r#"{"image":[]}"#,
            // No setting, or itself.
            r#"{"missing":true}"#,
            r#"{"caption":""}"#,
            r#"{}"#,
            // A single name still needs a boolean.
            r#""image""#,
            // More than four settings.
            r#"{"image":"logo","show":true,"size":1,"a":1,"b":2}"#,
        ] {
            let settings =
                format!(r#"[{base}, {{"type":"text","key":"caption","default":"","when":{bad}}}]"#);
            assert!(with(&settings).is_err(), "{bad}");
        }
        // A loop through values is a loop too.
        let looped = r#"[{"type":"choice","key":"a","default":"x","choices":["x","y"],"when":{"b":"y"}},
            {"type":"choice","key":"b","default":"x","choices":["x","y"],"when":{"a":"x"}}]"#;
        assert!(with(looped).is_err());
    }

    #[test]
    fn settings_are_read_and_checked_with_the_manifest() {
        let with = |settings: &str| {
            Manifest::parse(&GOOD.replace(
                "\"kind\": \"video\"",
                &format!("\"kind\": \"video\", \"options\": {settings}"),
            ))
        };
        let manifest = with(r#"[{"type":"boolean","key":"show_album","default":true}]"#).unwrap();
        assert_eq!(manifest.settings[0].key(), "show_album");

        for bad in [
            r#"[{"type":"boolean","key":"a","default":true},{"type":"boolean","key":"a","default":false}]"#,
            r#"[{"type":"choice","key":"c","default":"x","choices":["a"]}]"#,
            r#"[{"type":"boolean","key":"BAD","default":true}]"#,
        ] {
            assert!(
                matches!(
                    with(bad),
                    Err(ManifestError::Field {
                        field: "options",
                        ..
                    })
                ),
                "{bad}"
            );
        }
        let many: Vec<String> = (0..17)
            .map(|n| format!(r#"{{"type":"boolean","key":"k{n}","default":true}}"#))
            .collect();
        assert!(with(&format!("[{}]", many.join(","))).is_err());
        assert!(Manifest::parse(GOOD).unwrap().settings.is_empty());
    }

    #[test]
    fn the_description_of_who_wrote_it_is_checked() {
        let with = |extra: &str| {
            Manifest::parse(&GOOD.replace(
                "\"kind\": \"video\"",
                &format!("\"kind\": \"video\", {extra}"),
            ))
        };
        let good = with(
            r##""author": "manucabral", "contributors": ["Ana"], "color": "#A56786",
                "tags": ["video", "invented", "music"], "icon": "https://img.example.com/logo.png""##,
        )
        .unwrap();
        assert_eq!(good.author.as_deref(), Some("manucabral"));
        assert_eq!(good.tags, ["video", "music"]);
        assert!(with(r#""icon": "logo.webp""#).is_ok());
        assert!(with(r#""icon": "art/logo.PNG""#).is_ok());

        let bad = [
            (r#""color": "red""#, "color"),
            (r##""color": "#12345""##, "color"),
            (r##""color": "#GGGGGG""##, "color"),
            (r#""tags": ["Upper"]"#, "tags"),
            (r#""tags": ["a","b","c","d","e","f","g","h","i"]"#, "tags"),
            (r#""tags": [""]"#, "tags"),
            (r#""icon": "http://insecure.example.com/a.png""#, "icon"),
            (r#""icon": "../escape.png""#, "icon"),
            (r#""icon": "/absolute.png""#, "icon"),
            (r#""icon": "C:/logo.png""#, "icon"),
            (r#""icon": "script.js""#, "icon"),
            (r#""icon": "has space.png""#, "icon"),
            // A line break written as a JSON escape: valid JSON, refused as a name.
            ("\"author\": \"line\\nbreak\"", "author"),
            ("\"description\": \"line\\nbreak\"", "description"),
        ];
        for (extra, field) in bad {
            let error = with(extra).unwrap_err();
            assert!(
                matches!(error, ManifestError::Field { field: f, .. } if f == field),
                "{extra}"
            );
        }
        let many: Vec<String> = (0..11).map(|n| format!("\"p{n}\"")).collect();
        assert!(with(&format!("\"contributors\": [{}]", many.join(","))).is_err());
        let long = |n: usize| format!("\"description\": \"{}\"", "é".repeat(n));
        assert!(with(&long(MAX_DESCRIPTION)).is_ok());
        assert!(with(&long(MAX_DESCRIPTION + 1)).is_err());
    }

    #[test]
    fn other_names_are_short_and_different_from_each_other() {
        let with = |names: &str| {
            Manifest::parse(&GOOD.replace(
                "\"kind\": \"video\"",
                &format!("\"kind\": \"video\", \"other_names\": {names}"),
            ))
        };
        let good = with(r#"["VS", "Old Video"]"#).unwrap();
        assert_eq!(good.other_names, ["VS", "Old Video"]);
        assert!(Manifest::parse(GOOD).unwrap().other_names.is_empty());
        let long = format!("[\"{}\"]", "x".repeat(65));
        let nine: Vec<String> = (0..9).map(|n| format!("\"n{n}\"")).collect();
        for bad in [
            r#"[""]"#.to_owned(),
            r#"["  "]"#.to_owned(),
            r#"["video site"]"#.to_owned(),
            r#"["VS", "vs"]"#.to_owned(),
            "[\"line\\nbreak\"]".to_owned(),
            long,
            format!("[{}]", nine.join(",")),
        ] {
            let error = with(&bad).unwrap_err();
            assert!(
                matches!(
                    error,
                    ManifestError::Field {
                        field: "other_names",
                        ..
                    }
                ),
                "{bad}"
            );
        }
    }
}
