//! Test cases of a presence: what it read once and what it showed for that.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use rpp_plugins::engine::MAX_SNAPSHOT;
use rpp_plugins::{Host, Manifest, Plugin, Request};
use rpp_service::Presence;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::views::CaseResult;

/// Cases kept for each presence.
const MAX_CASES: usize = 50;
/// Longest name, in characters.
const MAX_NAME: usize = 60;
/// Largest file of cases read: every case at its largest, and then some.
const MAX_FILE: u64 = (MAX_CASES as u64 + 1) * 2 * MAX_SNAPSHOT as u64;

/// A snapshot and what the presence should show for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Case {
    /// How the author calls it.
    pub name: String,
    /// What `read` returned.
    pub snapshot: Value,
    /// What should be shown; null when the presence should show nothing.
    pub expected: Value,
}

/// Answers every call with an error: a case replays what was read before.
struct Offline;

impl Host for Offline {
    fn call(&self, _: Request) -> Result<Value, String> {
        Err("Test cases run without the computer or the network.".into())
    }
}

fn file(dir: &Path) -> PathBuf {
    dir.join("cases.json")
}

/// The saved cases; a file that is missing or unreadable gives none.
pub(crate) fn load(dir: &Path) -> Vec<Case> {
    let path = file(dir);
    if fs::metadata(&path).map_or(true, |meta| meta.len() > MAX_FILE) {
        return Vec::new();
    }
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write(dir: &Path, cases: &[Case]) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_vec_pretty(cases).map_err(|e| e.to_string())?;
    let path = file(dir);
    let temporary = dir.join("cases.json.tmp");
    fs::write(&temporary, text).map_err(|e| e.to_string())?;
    fs::rename(&temporary, path).map_err(|e| e.to_string())
}

/// Adds a case to the presence's folder `dir`, or replaces the one with its name.
///
/// Raises:
///     String: The name is empty or long, the snapshot is too large, there
///         are too many cases, or the file cannot be written.
pub(crate) fn put(dir: &Path, mut case: Case) -> Result<(), String> {
    case.name = case.name.trim().to_owned();
    if case.name.is_empty() || case.name.chars().count() > MAX_NAME {
        return Err(format!("A test name has 1 to {MAX_NAME} characters."));
    }
    if case.snapshot.to_string().len() > MAX_SNAPSHOT {
        return Err("This snapshot is too large to keep.".into());
    }
    let mut cases = load(dir);
    if let Some(same) = cases.iter_mut().find(|other| other.name == case.name) {
        *same = case;
    } else if cases.len() >= MAX_CASES {
        return Err(format!("A presence keeps up to {MAX_CASES} test cases."));
    } else {
        cases.push(case);
    }
    write(dir, &cases)
}

/// Removes a case; a name that is not saved changes nothing.
pub(crate) fn remove(dir: &Path, name: &str) -> Result<(), String> {
    let mut cases = load(dir);
    let before = cases.len();
    cases.retain(|case| case.name != name);
    if cases.len() == before {
        return Ok(());
    }
    write(dir, &cases)
}

fn plugin(manifest: &Manifest, presence: &Presence) -> Result<Plugin, String> {
    let mut plugin = Plugin::load(manifest.clone(), &presence.source, Rc::new(Offline))
        .map_err(|e| e.to_string())?;
    plugin
        .set_strings(&presence.strings)
        .map_err(|e| e.to_string())?;
    Ok(plugin)
}

/// Replays one case in a script of its own, so cases cannot affect each other.
fn check(presence: &Presence, case: &Case) -> CaseResult {
    let shown = plugin(&presence.manifest, presence).and_then(|mut plugin| {
        plugin
            .replay(&case.snapshot)
            .map_err(|error| error.to_string())
    });
    let actual = shown.map(|tick| {
        tick.activity
            .and_then(|activity| serde_json::to_value(activity).ok())
            .unwrap_or(Value::Null)
    });
    match actual {
        Ok(actual) => CaseResult {
            name: case.name.clone(),
            passed: actual == case.expected,
            expected: case.expected.clone(),
            actual,
            error: None,
        },
        Err(error) => CaseResult {
            name: case.name.clone(),
            passed: false,
            expected: case.expected.clone(),
            actual: Value::Null,
            error: Some(error),
        },
    }
}

/// Replays every case against the presence as it is now; one result per case,
/// in order.
pub(crate) fn run(presence: &Presence, cases: &[Case]) -> Vec<CaseResult> {
    cases.iter().map(|case| check(presence, case)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rpp_core::Activity;
    use serde_json::json;

    /// An activity as the inspector stores it, with every field.
    fn shown(details: &str) -> Value {
        serde_json::to_value(Activity {
            details: Some(details.into()),
            ..Activity::default()
        })
        .unwrap()
    }

    fn case(name: &str, title: &str) -> Case {
        Case {
            name: name.into(),
            snapshot: json!({"title": title}),
            expected: shown(title),
        }
    }

    fn presence(source: &str) -> Presence {
        Presence {
            manifest: Manifest::parse(
                r#"{"name":"Demo","version":"1","client_id":"12345678901234567",
                    "permissions":{"process":true}}"#,
            )
            .unwrap(),
            source: source.into(),
            strings: Default::default(),
            settings: Default::default(),
        }
    }

    const SCRIPT: &str = "function read(){ throw new Error('read must not run') }
        function activity(s){ return s.title ? {details: s.title} : null }";

    fn details(result: &CaseResult) -> &Value {
        &result.actual["details"]
    }

    #[test]
    fn cases_are_saved_replaced_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).is_empty());
        put(dir.path(), case("  Watching  ", "A")).unwrap();
        put(dir.path(), case("Other", "B")).unwrap();
        put(dir.path(), case("Watching", "C")).unwrap();
        let cases = load(dir.path());
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].snapshot["title"], "C");
        remove(dir.path(), "Watching").unwrap();
        remove(dir.path(), "Not saved").unwrap();
        assert_eq!(load(dir.path()).len(), 1);
    }

    #[test]
    fn names_snapshots_and_the_count_have_limits() {
        let dir = tempfile::tempdir().unwrap();
        assert!(put(dir.path(), case("   ", "A")).is_err());
        assert!(put(dir.path(), case(&"n".repeat(MAX_NAME + 1), "A")).is_err());
        let big = case("Big", &"x".repeat(MAX_SNAPSHOT));
        assert!(put(dir.path(), big).is_err());
        for number in 0..MAX_CASES {
            put(dir.path(), case(&number.to_string(), "A")).unwrap();
        }
        assert!(put(dir.path(), case("One too many", "A")).is_err());
        // Replacing one still works at the limit.
        put(dir.path(), case("0", "B")).unwrap();
    }

    #[test]
    fn a_damaged_file_gives_no_cases() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(file(dir.path()), "not json").unwrap();
        assert!(load(dir.path()).is_empty());
    }

    #[test]
    fn a_case_replays_only_what_it_saved() {
        let results = run(&presence(SCRIPT), &[case("Works", "Alpha")]);
        assert!(results[0].passed, "{results:?}");
        assert_eq!(details(&results[0]), "Alpha");
    }

    #[test]
    fn a_change_in_what_is_shown_fails_and_tells_both_sides() {
        let changed = "function read(){return {}}
            function activity(s){ return {details: s.title + '!'} }";
        let results = run(&presence(changed), &[case("Changed", "A")]);
        assert!(!results[0].passed);
        assert_eq!(results[0].expected["details"], "A");
        assert_eq!(details(&results[0]), "A!");
    }

    #[test]
    fn showing_nothing_can_be_expected() {
        let quiet = Case {
            name: "Nothing".into(),
            snapshot: json!({}),
            expected: Value::Null,
        };
        assert!(run(&presence(SCRIPT), &[quiet])[0].passed);
    }

    #[test]
    fn a_script_error_is_a_failure_with_its_message() {
        let broken = "function read(){return {}} function activity(){ throw new Error('boom') }";
        let results = run(&presence(broken), &[case("Broken", "A")]);
        assert!(!results[0].passed);
        assert!(results[0].error.as_deref().unwrap().contains("boom"));
    }

    #[test]
    fn a_script_cannot_reach_the_computer_from_a_case() {
        let curious = "function read(){return {}}
            function activity(){ return {details: 'x' + host.process.list().length} }";
        let results = run(&presence(curious), &[case("Curious", "A")]);
        assert!(!results[0].passed);
        let error = results[0].error.as_deref().unwrap();
        assert!(error.contains("without the computer"), "{error}");
    }

    #[test]
    fn cases_do_not_share_what_the_script_remembers() {
        let counting = "let seen = 0; function read(){return {}}
            function activity(){ seen += 1; return {details: 'seen ' + seen} }";
        let expected = Case {
            name: "First".into(),
            snapshot: json!({}),
            expected: shown("seen 1"),
        };
        let second = Case {
            name: "Second".into(),
            ..expected.clone()
        };
        let results = run(&presence(counting), &[expected, second]);
        assert!(results.iter().all(|result| result.passed), "{results:?}");
    }
}
