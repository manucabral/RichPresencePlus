//! A presence for a program, made without writing code.

use std::path::{Path, PathBuf};

use rpp_plugins::Manifest;
use serde::Deserialize;
use serde_json::json;

use crate::templates;

/// Discord application shared by presences made here. Discord shows its name
/// as the title.
pub(crate) const SHARED_CLIENT_ID: &str = "1567580382377803786";

/// Longest line Discord accepts.
const MAX_TEXT: usize = 128;
/// Longest program name.
const MAX_PROGRAM: usize = 96;

/// Programs that belong to Windows and say nothing about what someone does.
const SYSTEM: [&str; 16] = [
    "applicationframehost.exe",
    "conhost.exe",
    "csrss.exe",
    "ctfmon.exe",
    "dllhost.exe",
    "dwm.exe",
    "explorer.exe",
    "fontdrvhost.exe",
    "lsass.exe",
    "rundll32.exe",
    "runtimebroker.exe",
    "searchhost.exe",
    "services.exe",
    "sihost.exe",
    "svchost.exe",
    "taskhostw.exe",
];

/// What the user chose.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProgramPresence {
    /// Name of the presence.
    pub name: String,
    /// Executable to look for, such as `code.exe`.
    pub program: String,
    /// First line on Discord.
    pub details: String,
    /// Second line on Discord; may be empty.
    pub state: String,
}

/// The running programs worth offering: lower-case executable names, sorted,
/// each once, without Windows' own.
pub(crate) fn useful_programs(names: &[String]) -> Vec<String> {
    let mut programs: Vec<String> = names
        .iter()
        .map(|name| name.to_lowercase())
        .filter(|name| valid_program(name) && !SYSTEM.contains(&name.as_str()))
        .collect();
    programs.sort();
    programs.dedup();
    programs
}

fn valid_program(name: &str) -> bool {
    name.len() <= MAX_PROGRAM
        && name.len() > 4
        && name.to_lowercase().ends_with(".exe")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" ._-()+".contains(&b))
}

fn script(name: &str, program: &str, details: &str, state: &str) -> Result<String, String> {
    let text = |value: &str| serde_json::to_string(value).map_err(|e| e.to_string());
    Ok(format!(
        r#"// Made by Rich Presence Plus. You can edit this file.

const NAME = {name};
const PROGRAM = {program};
const DETAILS = {details};
const STATE = {state};

// Unix seconds at which the program was first seen; 0 while it is closed.
let since = 0;

function read(context) {{
  const running = host.process.list().some((name) => name.toLowerCase() === PROGRAM);
  if (!running) {{
    since = 0;
    return null;
  }}
  if (since === 0) since = context.now;
  return {{ since }};
}}

function activity(seen) {{
  return {{ name: NAME, details: DETAILS, state: STATE || null, kind: "playing", start: seen.since }};
}}

function meta() {{
  return {{ application: PROGRAM }};
}}
"#,
        name = text(name)?,
        program = text(program)?,
        details = text(details)?,
        state = text(state)?,
    ))
}

/// Writes the presence folder and returns it.
///
/// Raises:
///     String: The program or a text cannot be used, the name is taken, or
///         the files cannot be written.
pub(crate) fn create(
    root: &Path,
    choice: &ProgramPresence,
    author: Option<&str>,
) -> Result<PathBuf, String> {
    let program = choice.program.trim().to_lowercase();
    if !valid_program(&program) {
        return Err("Choose a program that ends in .exe.".into());
    }
    let details = choice.details.trim();
    let state = choice.state.trim();
    let length = details.chars().count();
    if !(2..=MAX_TEXT).contains(&length) || state.chars().count() > MAX_TEXT {
        return Err(format!("Each line takes 2 to {MAX_TEXT} characters."));
    }
    let (name, folder) = templates::checked(&choice.name)?;
    let manifest = json!({
        "name": name,
        "version": "1.0.0",
        "description": format!("Shows that {program} is open."),
        "client_id": SHARED_CLIENT_ID,
        "kind": "other",
        "interval": 10,
        "entry": "main.js",
        "permissions": { "process": true },
        "author": author,
    });
    let manifest = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n";
    Manifest::parse(&manifest).map_err(|e| e.to_string())?;
    let source = script(name, &program, details, state)?;
    templates::publish(root, &folder, |staging| {
        templates::write(
            staging,
            &[("manifest.json", &manifest), ("main.js", &source)],
        )
        .map_err(|e| format!("Cannot write the presence: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rpp_plugins::{Host, Plugin, Request};
    use serde_json::Value;
    use std::fs;
    use std::rc::Rc;

    struct Running(&'static [&'static str]);

    impl Host for Running {
        fn call(&self, request: Request) -> Result<Value, String> {
            assert!(matches!(request, Request::ProcessList));
            Ok(json!(self.0))
        }
    }

    fn choice(details: &str) -> ProgramPresence {
        ProgramPresence {
            name: "My Editor".into(),
            program: "Code.exe".into(),
            details: details.into(),
            state: "On a project".into(),
        }
    }

    fn load(folder: &Path, running: &'static [&'static str]) -> Plugin {
        let manifest =
            Manifest::parse(&fs::read_to_string(folder.join("manifest.json")).unwrap()).unwrap();
        let source = fs::read_to_string(folder.join("main.js")).unwrap();
        Plugin::load(manifest, &source, Rc::new(Running(running))).unwrap()
    }

    #[test]
    fn the_presence_shows_while_the_program_runs() {
        let root = tempfile::tempdir().unwrap();
        let folder = create(root.path(), &choice("Writing code"), Some("Ana")).unwrap();
        let tick = load(&folder, &["CODE.EXE"])
            .poll(&json!({"now": 1000, "pages": [], "options": {}}))
            .unwrap();
        let shown = tick.activity.unwrap();
        assert_eq!(shown.details.as_deref(), Some("Writing code"));
        assert_eq!(shown.state.as_deref(), Some("On a project"));
        assert_eq!(shown.start, Some(1000));
        assert_eq!(tick.meta.application, "code.exe");

        let quiet = load(&folder, &["other.exe"])
            .poll(&json!({"now": 1000, "pages": [], "options": {}}))
            .unwrap();
        assert!(quiet.activity.is_none());
    }

    #[test]
    fn typed_text_stays_text() {
        let root = tempfile::tempdir().unwrap();
        let attack = r#"x"; host.process.list(); throw new Error("no"); ""#;
        let folder = create(root.path(), &choice(attack), None).unwrap();
        let tick = load(&folder, &["code.exe"])
            .poll(&json!({"now": 1, "pages": [], "options": {}}))
            .unwrap();
        assert_eq!(tick.activity.unwrap().details.as_deref(), Some(attack));
    }

    #[test]
    fn bad_choices_are_refused_and_write_nothing() {
        let root = tempfile::tempdir().unwrap();
        let mut bad = choice("Fine");
        bad.program = r"..\evil.bat".into();
        assert!(create(root.path(), &bad, None).is_err());
        assert!(create(root.path(), &choice("x"), None).is_err());
        assert!(create(root.path(), &choice(&"y".repeat(MAX_TEXT + 1)), None).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn windows_own_programs_are_not_offered() {
        let names: Vec<String> = [
            "svchost.exe",
            "Code.exe",
            "code.exe",
            "System",
            "Game.EXE",
            "dwm.exe",
        ]
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
        assert_eq!(useful_programs(&names), ["code.exe", "game.exe"]);
    }
}
