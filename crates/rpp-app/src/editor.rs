//! What a code editor needs to explain a presence while someone writes it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rpp_plugins::{Extractor, Manifest, Setting};

/// Name of the file that puts a folder of JavaScript under the editor's checks.
pub(crate) const JSCONFIG: &str = "jsconfig.json";

const JSCONFIG_TEXT: &str = r#"{
  "compilerOptions": { "checkJs": true, "strict": false, "target": "ES2023", "lib": ["ES2023"] },
  "include": ["main.js", ".vscode/*.d.ts"]
}
"#;

const SETTINGS: &str = r#"{
  "json.schemas": [
    { "fileMatch": ["/manifest.json"], "url": "./.vscode/manifest.schema.json" }
  ],
  "files.exclude": { ".vscode": true, "jsconfig.json": true }
}
"#;

/// Where Visual Studio Code is, among the places its installers use.
///
/// Args:
///     user_programs: `%LOCALAPPDATA%\\Programs`.
///     programs: `%ProgramFiles%`.
pub fn visual_studio_code(
    user_programs: Option<&Path>,
    programs: Option<&Path>,
) -> Option<PathBuf> {
    [user_programs, programs]
        .into_iter()
        .flatten()
        .map(|folder| folder.join("Microsoft VS Code").join("Code.exe"))
        .find(|program| program.is_file())
}

fn write_if_changed(path: &Path, text: &str) -> io::Result<()> {
    if fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

/// Makes a text safe inside a `/** */` comment.
fn comment(text: &str) -> String {
    text.replace("*/", "* /")
}

fn value_type(extractor: &Extractor) -> (&'static str, String) {
    match extractor {
        Extractor::Text { selector } => ("string | null", format!("Text of `{selector}`.")),
        Extractor::Attr { selector, name } => (
            "string | null",
            format!("Attribute `{name}` of `{selector}`."),
        ),
        Extractor::Meta { name } => (
            "string | null",
            format!("Content of the meta tag `{name}`."),
        ),
        Extractor::Media { selector } => (
            "Player | null",
            format!(
                "The player `{}`.",
                selector.as_deref().unwrap_or("video, audio")
            ),
        ),
        Extractor::List { selector, .. } => ("string[]", format!("Every match of `{selector}`.")),
        Extractor::Variable { path } => ("any", format!("The page variable `{path}`.")),
        Extractor::Json { selector, path } => (
            "any",
            format!(
                "JSON in `{selector}`, at `{}`.",
                path.as_deref().unwrap_or("its root")
            ),
        ),
        Extractor::Storage { key, .. } => ("any", format!("The stored entry `{key}`.")),
        Extractor::Style { selector, name } => {
            ("string | null", format!("Style `{name}` of `{selector}`."))
        }
        Extractor::Session => (
            "Session | null",
            "What the site publishes as playing.".to_owned(),
        ),
        Extractor::Response { url, path } => (
            "any",
            format!(
                "The last answer from `{url}`, at `{}`.",
                path.as_deref().unwrap_or("its root")
            ),
        ),
    }
}

fn option_type(setting: &Setting) -> String {
    match setting {
        Setting::Boolean { .. } => "boolean".to_owned(),
        Setting::Number { .. } => "number".to_owned(),
        Setting::Text { .. } => "string".to_owned(),
        Setting::Choice { choices, .. } => choices
            .iter()
            .map(|choice| serde_json::to_string(choice).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

/// The declaration file with the types that belong to one presence.
pub(crate) fn types_of(manifest: &Manifest) -> String {
    let mut text = String::from(
        "// Written by Rich Presence Plus from manifest.json each time it changes.\n\n/** What this presence reads from a page, by the names in its manifest. */\ninterface Values {\n",
    );
    if let Some(page) = &manifest.page {
        let frames = page.frames.iter().flat_map(|frames| &frames.extract);
        for (name, extractor) in page.extract.iter().chain(frames) {
            let (kind, about) = value_type(extractor);
            text.push_str(&format!(
                "  /** {} */\n  {name}: {kind};\n",
                comment(&about)
            ));
        }
    }
    text.push_str("}\n\n/** The options of this presence, always complete and valid. */\ninterface Options {\n");
    for setting in &manifest.settings {
        let key = serde_json::to_string(setting.key()).unwrap_or_default();
        text.push_str(&format!("  {key}: {};\n", option_type(setting)));
    }
    text.push_str("}\n\n/** What the permissions of this presence give. Anything else does not exist. */\ndeclare const host: {\n");
    let granted = &manifest.permissions;
    if granted.process || !granted.args.is_empty() {
        let mut process = Vec::new();
        if granted.process {
            process.push("list(): string[]");
        }
        if !granted.args.is_empty() {
            process.push("args(name: string): Record<string, string | true> | null");
        }
        text.push_str(&format!("  process: {{ {} }};\n", process.join("; ")));
    }
    if granted.window {
        text.push_str("  window: HostWindow;\n");
    }
    if granted.media {
        text.push_str("  media: HostMedia;\n");
    }
    if !granted.http.is_empty() {
        text.push_str("  http: HostHttp;\n");
    }
    if !granted.files.is_empty() {
        text.push_str("  files: HostFiles;\n");
    }
    if !granted.query.is_empty() {
        text.push_str("  query: HostQuery;\n");
    }
    text.push_str("};\n");
    text
}

/// The name of a function's first parameter, when it is a plain name.
fn first_parameter(line: &str) -> Option<&str> {
    let inside = line.split_once('(')?.1;
    let name = inside.split([',', ')']).next()?.trim();
    (!name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$'))
    .then_some(name)
}

/// Tells the editor what the functions of a script receive.
pub(crate) fn annotate(source: &str) -> Option<String> {
    const SEEN: &str = "NonNullable<ReturnType<typeof read>>";
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let lines: Vec<&str> = source.split(newline).collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 6);
    let mut changed = false;
    for (index, line) in lines.iter().enumerate() {
        let documented = index > 0 && lines[index - 1].trim_end().ends_with("*/");
        let parameter = first_parameter(line);
        let note = if documented {
            None
        } else if line.starts_with("function read(") {
            parameter.map(|name| format!("/** @param {{Context}} {name} */"))
        } else if line.starts_with("function activity(") {
            Some(match parameter {
                Some(name) => format!(
                    "/**{newline} * @param {{{SEEN}}} {name}{newline} * @returns {{Activity | null}}{newline} */"
                ),
                None => "/** @returns {Activity | null} */".to_owned(),
            })
        } else if line.starts_with("function meta(") {
            Some(match parameter {
                Some(name) => format!(
                    "/**{newline} * @param {{{SEEN}}} {name}{newline} * @returns {{Meta}}{newline} */"
                ),
                None => "/** @returns {Meta} */".to_owned(),
            })
        } else {
            None
        };
        if let Some(note) = note {
            out.push(note);
            changed = true;
        }
        out.push((*line).to_owned());
    }
    changed.then(|| out.join(newline))
}

/// Writes the types of a presence after its manifest changed.
pub(crate) fn refresh_types(folder: &Path, manifest: &Manifest) -> io::Result<()> {
    write_if_changed(
        &folder.join(".vscode/presence-types.d.ts"),
        &types_of(manifest),
    )
}

/// Gives a presence the person owns everything the editor reads.
pub(crate) fn prepare(folder: &Path, manifest: &Manifest) -> io::Result<()> {
    write_if_changed(&folder.join(JSCONFIG), JSCONFIG_TEXT)?;
    write_if_changed(&folder.join(".vscode/settings.json"), SETTINGS)?;
    write_if_changed(
        &folder.join(".vscode/manifest.schema.json"),
        include_str!("../templates/manifest.schema.json"),
    )?;
    write_if_changed(
        &folder.join(".vscode/script-api.d.ts"),
        include_str!("../templates/script-api.d.ts"),
    )?;
    refresh_types(folder, manifest)?;
    let script = folder.join(&manifest.entry);
    if let Some(annotated) = fs::read_to_string(&script)
        .ok()
        .and_then(|text| annotate(&text))
    {
        fs::write(script, annotated)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest::parse(
            r#"{
              "name": "Mine", "version": "1.0.0", "client_id": "12345678901234567",
              "permissions": { "window": true, "args": { "game.exe": ["-h"] } },
              "page": {
                "matches": ["*://a.example/*"],
                "extract": {
                  "title": { "from": "text", "selector": "h1" },
                  "player": { "from": "media" },
                  "genres": { "from": "list", "selector": "a.genre" }
                },
                "frames": { "matches": ["*://b.example/*"], "extract": { "inner": { "from": "text", "selector": "h2" } } }
              },
              "options": [
                { "type": "boolean", "key": "show_title", "default": true },
                { "type": "choice", "key": "image", "default": "logo", "choices": ["logo", "cover"] }
              ]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn the_types_name_what_this_presence_declares() {
        let text = types_of(&manifest());
        assert!(text.contains("title: string | null;"), "{text}");
        assert!(text.contains("/** Text of `h1`. */"), "{text}");
        assert!(text.contains("player: Player | null;"), "{text}");
        assert!(text.contains("genres: string[];"), "{text}");
        assert!(
            text.contains("inner: string | null;"),
            "a frame is a value like any other"
        );
        assert!(text.contains("\"show_title\": boolean;"), "{text}");
        assert!(text.contains("\"image\": \"logo\" | \"cover\";"), "{text}");
    }

    /// What the editor offers must be what the script gets, or it teaches the wrong thing.
    #[test]
    fn host_holds_only_what_the_permissions_give() {
        let text = types_of(&manifest());
        assert!(text.contains("window: HostWindow;"), "{text}");
        assert!(text.contains("args(name: string)"), "{text}");
        assert!(
            !text.contains("list()"),
            "no permission to list programs: {text}"
        );
        for missing in ["HostMedia", "HostHttp", "HostFiles", "HostQuery"] {
            assert!(!text.contains(missing), "{missing}");
        }
    }

    #[test]
    fn a_selector_cannot_end_the_comment() {
        let mut manifest = manifest();
        manifest.page = serde_json::from_value(serde_json::json!({
            "matches": ["*://a.example/*"],
            "extract": { "title": { "from": "text", "selector": "a */ b" } }
        }))
        .unwrap();
        assert!(!types_of(&manifest).contains("a */ b"));
    }

    #[test]
    fn each_function_learns_what_it_receives_once() {
        let source = "let x = 0;\n\nfunction read(context) {\n}\n\nfunction activity(site) {\n}\n\nfunction meta() {\n}\n";
        let annotated = annotate(source).unwrap();
        assert!(
            annotated.contains("/** @param {Context} context */\nfunction read(context) {"),
            "{annotated}"
        );
        assert!(
            annotated.contains("@param {NonNullable<ReturnType<typeof read>>} site"),
            "{annotated}"
        );
        assert!(
            annotated.contains("/** @returns {Meta} */\nfunction meta() {"),
            "{annotated}"
        );
        assert_eq!(annotate(&annotated), None, "a second pass changes nothing");
    }

    #[test]
    fn a_comment_of_the_person_is_left_alone() {
        let source = "/** Mine. */\nfunction read(context) {\n}\n";
        assert_eq!(annotate(source), None);
        assert_eq!(annotate("const read = () => null;\n"), None);
    }

    #[test]
    fn windows_line_ends_stay() {
        let annotated = annotate("function read(c) {\r\n}\r\n").unwrap();
        assert_eq!(
            annotated,
            "/** @param {Context} c */\r\nfunction read(c) {\r\n}\r\n"
        );
    }

    #[test]
    fn a_second_preparation_writes_nothing() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("main.js"), "function read(c) {\n}\n").unwrap();
        prepare(folder.path(), &manifest()).unwrap();
        for file in [
            "jsconfig.json",
            ".vscode/settings.json",
            ".vscode/script-api.d.ts",
            ".vscode/presence-types.d.ts",
        ] {
            assert!(folder.path().join(file).is_file(), "{file}");
        }
        let times = |path: &str| {
            fs::metadata(folder.path().join(path))
                .unwrap()
                .modified()
                .unwrap()
        };
        let before = (
            times("main.js"),
            times(".vscode/presence-types.d.ts"),
            times("jsconfig.json"),
        );
        prepare(folder.path(), &manifest()).unwrap();
        let after = (
            times("main.js"),
            times(".vscode/presence-types.d.ts"),
            times("jsconfig.json"),
        );
        assert_eq!(before, after);
    }
}
