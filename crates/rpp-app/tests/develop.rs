//! The developer tools: create, copy, reload, inspect and hot reload.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use rpp_app::{Case, Controller, Options, Template};
use rpp_testkit::{Discord, Processes, eventually};

/// A user folder (where presences are written) and a bundled one.
struct Setup {
    data: tempfile::TempDir,
    user: tempfile::TempDir,
    bundled: tempfile::TempDir,
}

fn setup() -> Setup {
    Setup {
        data: tempfile::tempdir().unwrap(),
        user: tempfile::tempdir().unwrap(),
        bundled: tempfile::tempdir().unwrap(),
    }
}

/// Saves, compares and restores through the controller, and the running
/// presence follows.
#[tokio::test(flavor = "multi_thread")]
async fn a_saved_version_comes_back_and_the_presence_reloads() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Old"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;

    let saved = controller.save_version("Mine", "first").unwrap();
    assert_eq!((saved.id, saved.version.as_str()), (1, "1"));
    assert!(controller.save_version("Mine", "again").is_err());

    let script_path = setup.user.path().join("mine/main.js");
    fs::write(&script_path, script("New")).unwrap();
    let changes = controller.compare_version("Mine", 1).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "main.js");

    controller.restore_version("Mine", 1).unwrap();
    assert!(fs::read_to_string(&script_path).unwrap().contains("Old"));
    assert_eq!(controller.presence_versions("Mine").unwrap().len(), 2);
    // Kept apart from the presence, so it never travels with it.
    assert!(setup.data.path().join("workbench/mine/versions").is_dir());
    assert!(!setup.user.path().join("mine/versions").exists());
    controller.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cases_replay_what_the_inspector_saw() {
    let setup = setup();
    let source = "function read(){return {title:'Saved'}}
        function activity(s){return {details:s.title}}";
    presence(setup.user.path(), "mine", "Mine", source);
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;

    let seen = controller.inspect("Mine").unwrap();
    let expected = serde_json::to_value(seen.activity.unwrap()).unwrap();
    let case = Case {
        name: "Title".into(),
        snapshot: seen.snapshot.unwrap(),
        expected,
    };
    controller.save_test_case("Mine", case).unwrap();
    assert_eq!(controller.test_cases("Mine").unwrap().len(), 1);
    assert!(controller.run_test_cases("Mine").unwrap()[0].passed);

    // The script changes on disk and the running copy with it.
    let edited = "function read(){return {title:'Saved'}}
        function activity(s){return {details:s.title + ' edited'}}";
    fs::write(setup.user.path().join("mine/main.js"), edited).unwrap();
    controller.reload_presences();
    let results = controller.run_test_cases("Mine").unwrap();
    assert!(!results[0].passed);
    assert_eq!(results[0].actual["details"], "Saved edited");

    controller.delete_test_case("Mine", "Title").unwrap();
    assert!(controller.test_cases("Mine").unwrap().is_empty());
    controller.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_presence_that_breaks_a_rule_is_not_prepared() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Hi"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    let checks = controller.review_presence("Mine").unwrap();
    assert!(checks.iter().any(|check| !check.passed));
    assert!(controller.prepare_presence("Mine").is_err());
    assert!(!setup.data.path().join("exports").join("mine").exists());
    controller.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_tools_refuse_a_presence_the_user_did_not_write() {
    let setup = setup();
    presence(setup.bundled.path(), "theirs", "Theirs", &script("Hi"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    assert!(controller.save_version("Theirs", "").is_err());
    assert!(controller.review_presence("Theirs").is_err());
    assert!(controller.prepare_presence("Theirs").is_err());
    assert!(controller.run_test_cases("Theirs").is_err());
    assert!(controller.save_version("Nobody", "").is_err());
    controller.shutdown().await;
}

fn script(details: &str) -> String {
    format!("function read(){{return {{}}}} function activity(){{return {{details:'{details}'}}}}")
}

/// Writes a presence folder with a manifest and the given script.
fn presence(root: &Path, folder: &str, name: &str, source: &str) {
    let directory = root.join(folder);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("manifest.json"),
        format!(
            r#"{{"name":"{name}","version":"1","client_id":"12345678901234567","interval":1}}"#
        ),
    )
    .unwrap();
    fs::write(directory.join("main.js"), source).unwrap();
}

async fn start(setup: &Setup, discord: &Discord) -> Arc<Controller<Processes>> {
    let options = Options {
        data_dir: setup.data.path().to_owned(),
        presence_roots: vec![
            setup.user.path().to_owned(),
            setup.bundled.path().to_owned(),
        ],
        version: "1".into(),
        port_override: Some(0),
        system_language: "en".into(),
        catalog: None,
    };
    let controller = Controller::start(options, discord.clone(), Processes(vec![]))
        .await
        .unwrap();
    Arc::new(controller)
}

async fn finish(controller: Arc<Controller<Processes>>) {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(controller.shutdown());
    });
}

fn names(controller: &Controller<Processes>) -> Vec<String> {
    controller.presences().into_iter().map(|p| p.name).collect()
}

fn shows(discord: &Discord, text: &str) -> bool {
    discord.details().last() == Some(&Some(text.to_owned()))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_presence_from_a_template_appears_and_names_are_checked() {
    let setup = setup();
    let controller = start(&setup, &Discord::default()).await;
    assert!(names(&controller).is_empty());

    let folder = controller
        .create_presence("My Site", Template::Web)
        .unwrap();
    assert!(folder.ends_with("my-site"));
    assert_eq!(names(&controller), ["My Site"]);
    controller
        .create_presence("My Game", Template::Desktop)
        .unwrap();
    assert_eq!(names(&controller), ["My Game", "My Site"]);

    assert!(
        controller
            .create_presence("my site", Template::Web)
            .is_err()
    );
    assert!(controller.create_presence("../x", Template::Web).is_err());
    assert_eq!(names(&controller).len(), 2);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn inspecting_shows_what_a_presence_reads_without_publishing_it() {
    let setup = setup();
    presence(
        setup.user.path(),
        "demo",
        "Demo",
        "function read(c){ console.log('reading', c.options); return {n: 1}; } \
         function activity(s){ return {details: 'ONLY INSPECTED ' + s.n}; }",
    );
    // Turned off before the application starts, so it never runs: the only
    // way to see its text is to inspect it.
    let mut settings = rpp_config::Settings::default();
    settings.disabled.insert("Demo".into());
    rpp_config::SettingsStore::new(setup.data.path())
        .save(&settings)
        .unwrap();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    assert!(!controller.presences()[0].running);

    let seen = tokio::task::block_in_place(|| controller.inspect("Demo")).unwrap();
    assert_eq!(seen.snapshot, Some(serde_json::json!({"n": 1})));
    assert_eq!(
        seen.activity.unwrap().details.as_deref(),
        Some("ONLY INSPECTED 1")
    );
    assert_eq!(seen.error, None);
    assert_eq!(seen.console, ["reading {}"]);
    assert!(controller.inspect("Missing").is_err());
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    assert!(
        discord
            .details()
            .iter()
            .flatten()
            .all(|text| !text.contains("INSPECTED")),
        "inspecting never publishes"
    );
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_script_that_fails_is_reported_not_raised() {
    let setup = setup();
    presence(
        setup.user.path(),
        "bad",
        "Bad",
        "function read(){ throw new Error('boom') } function activity(){}",
    );
    presence(setup.user.path(), "worse", "Worse", "function (");
    let controller = start(&setup, &Discord::default()).await;
    let seen = tokio::task::block_in_place(|| controller.inspect("Bad")).unwrap();
    assert!(seen.error.unwrap().contains("boom"));
    assert!(seen.activity.is_none());
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn reloading_applies_edits_new_folders_and_removals() {
    let setup = setup();
    presence(setup.user.path(), "demo", "Demo", &script("Before"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    eventually("the first text", || shows(&discord, "Before")).await;

    fs::write(setup.user.path().join("demo/main.js"), script("After")).unwrap();
    presence(setup.user.path(), "extra", "Extra", &script("Extra text"));
    assert!(controller.reload_presences().is_empty());
    eventually("the edit", || shows(&discord, "After")).await;
    assert_eq!(names(&controller), ["Demo", "Extra"]);
    assert!(controller.presences().iter().all(|p| p.running));

    fs::remove_dir_all(setup.user.path().join("extra")).unwrap();
    controller.reload_presences();
    assert_eq!(names(&controller), ["Demo"]);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_folder_is_reported_when_reloading() {
    let setup = setup();
    let controller = start(&setup, &Discord::default()).await;
    fs::create_dir_all(setup.user.path().join("broken")).unwrap();
    fs::write(setup.user.path().join("broken/manifest.json"), "not json").unwrap();
    let problems = controller.reload_presences();
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].folder, "broken");
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn saving_a_watched_presence_applies_it_and_a_typo_keeps_the_old_version() {
    let setup = setup();
    presence(setup.user.path(), "demo", "Demo", &script("Version one"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    eventually("version one", || shows(&discord, "Version one")).await;
    controller.watch(Some("Demo")).unwrap();
    assert_eq!(controller.dev_status().watching.as_deref(), Some("Demo"));
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;

    let main = setup.user.path().join("demo/main.js");
    fs::write(&main, script("Version two")).unwrap();
    eventually("version two without a reload call", || {
        shows(&discord, "Version two")
    })
    .await;
    assert!(controller.dev_status().last_reload.is_some());
    assert_eq!(controller.dev_status().error, None);

    // A typo: the running version stays and the error is recorded.
    fs::write(&main, "function read( {").unwrap();
    eventually("the error", || controller.dev_status().error.is_some()).await;
    assert!(shows(&discord, "Version two"));
    assert!(controller.presences()[0].running);

    // Fixing it clears the error and applies the fix.
    fs::write(&main, script("Version three")).unwrap();
    eventually("the fix", || shows(&discord, "Version three")).await;
    assert_eq!(controller.dev_status().error, None);

    controller.watch(None).unwrap();
    assert_eq!(controller.dev_status().watching, None);
    fs::write(&main, script("Version four, not watched")).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1800)).await;
    assert!(shows(&discord, "Version three"));
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_renamed_presence_keeps_being_watched() {
    let setup = setup();
    presence(setup.user.path(), "demo", "Demo", &script("Old name"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    controller.watch(Some("Demo")).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;

    presence(setup.user.path(), "demo", "Renamed", &script("New name"));
    eventually("the new name runs", || shows(&discord, "New name")).await;
    assert_eq!(names(&controller), ["Renamed"]);
    assert_eq!(controller.dev_status().watching.as_deref(), Some("Renamed"));

    presence(
        setup.user.path(),
        "demo",
        "Renamed",
        &script("Edited again"),
    );
    eventually("the next edit", || shows(&discord, "Edited again")).await;
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn only_presences_in_the_users_folder_can_be_watched_but_any_can_be_copied() {
    let setup = setup();
    presence(
        setup.bundled.path(),
        "stock",
        "Stock",
        &script("Stock text"),
    );
    let controller = start(&setup, &Discord::default()).await;
    assert!(controller.watch(Some("Stock")).is_err());
    assert!(controller.watch(Some("Missing")).is_err());
    assert_eq!(controller.dev_status().watching, None);

    let folder = controller.copy_presence("Stock", "My Stock").unwrap();
    assert!(folder.ends_with("my-stock"));
    assert_eq!(names(&controller), ["My Stock", "Stock"]);
    controller.watch(Some("My Stock")).unwrap();
    assert!(controller.copy_presence("Stock", "my stock").is_err());
    assert!(controller.copy_presence("Missing", "Anything").is_err());
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn editing_the_description_of_a_presence_reloads_it_with_the_new_values() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Mine text"));
    presence(
        setup.bundled.path(),
        "stock",
        "Stock",
        &script("Stock text"),
    );
    let controller = start(&setup, &Discord::default()).await;

    let changes = rpp_app::ManifestChanges {
        author: Some("Ana".into()),
        color: Some("#A56786".into()),
        tags: Some(vec!["video".into()]),
        version: Some("2.0.0".into()),
        ..Default::default()
    };
    controller.update_manifest("Mine", &changes).unwrap();
    let view = &controller
        .presences()
        .into_iter()
        .find(|p| p.name == "Mine")
        .unwrap();
    assert_eq!(view.author.as_deref(), Some("Ana"));
    assert_eq!(view.color.as_deref(), Some("#A56786"));
    assert_eq!(view.tags, ["video"]);
    assert_eq!(view.version, "2.0.0");
    assert!(view.running, "it keeps running after the edit");

    // The file on disk has it too, so the edit survives a restart.
    let text = fs::read_to_string(setup.user.path().join("mine/manifest.json")).unwrap();
    assert!(text.contains("\"author\": \"Ana\""));

    let bad = rpp_app::ManifestChanges {
        color: Some("red".into()),
        ..Default::default()
    };
    assert!(controller.update_manifest("Mine", &bad).is_err());
    assert!(controller.update_manifest("Stock", &changes).is_err());
    assert!(controller.update_manifest("Missing", &changes).is_err());
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_name_cannot_be_taken_twice_whatever_its_case_or_where_the_first_one_lives() {
    let setup = setup();
    presence(setup.bundled.path(), "stock", "YouTube", &script("Stock"));
    presence(setup.user.path(), "mine", "Mine", &script("Mine"));
    let controller = start(&setup, &Discord::default()).await;

    for name in ["youtube", "YOUTUBE", " YouTube ", "mine", "MINE"] {
        let error = controller.create_presence(name, Template::Web).unwrap_err();
        assert!(error.contains("already exists"), "{name}: {error}");
        let copy = controller.copy_presence("Mine", name).unwrap_err();
        assert!(copy.contains("already exists"), "{name}: {copy}");
    }
    let error = controller
        .create_presence("con", Template::Web)
        .unwrap_err();
    assert!(error.contains("reserved"), "{error}");
    assert_eq!(names(&controller), ["Mine", "YouTube"]);
    assert_eq!(fs::read_dir(setup.user.path()).unwrap().count(), 1);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_folder_with_the_name_of_a_bundled_presence_does_not_replace_it() {
    let setup = setup();
    presence(setup.bundled.path(), "stock", "YouTube", &script("Stock"));
    // The user's folder is read first and its folder name sorts first, and still loses.
    presence(setup.user.path(), "aaa", "youtube", &script("Impostor"));
    presence(setup.user.path(), "mine", "Mine", &script("Mine"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;

    let list = controller.presences();
    let youtube = list.iter().find(|p| p.name == "YouTube").unwrap();
    assert_eq!(youtube.origin, rpp_app::Origin::Bundled);
    assert!(!youtube.editable);
    assert!(list.iter().all(|p| p.name != "youtube"));
    let problems = controller.overview().problems;
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].folder, "aaa");
    assert!(problems[0].message.contains("already used by YouTube"));

    let mine = list.iter().find(|p| p.name == "Mine").unwrap();
    assert_eq!(mine.origin, rpp_app::Origin::Local);
    assert!(mine.editable);
    // Only one presence is shown at a time, so turn the other one off.
    controller.set_enabled("Mine", false).unwrap();
    eventually("the bundled one runs", || {
        discord.details().contains(&Some("Stock".to_owned()))
    })
    .await;
    assert!(!discord.details().contains(&Some("Impostor".to_owned())));
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn bundled_presences_cannot_be_edited_or_watched_and_their_files_stay_as_they_were() {
    let setup = setup();
    presence(setup.bundled.path(), "stock", "Stock", &script("Stock"));
    let controller = start(&setup, &Discord::default()).await;
    let manifest = setup.bundled.path().join("stock/manifest.json");
    let before = fs::read_to_string(&manifest).unwrap();

    let changes = rpp_app::ManifestChanges {
        author: Some("me".into()),
        ..Default::default()
    };
    let error = controller.update_manifest("Stock", &changes).unwrap_err();
    assert!(error.contains("Only presences you wrote"), "{error}");
    let error = controller.watch(Some("Stock")).unwrap_err();
    assert!(error.contains("Only presences you wrote"), "{error}");
    assert_eq!(fs::read_to_string(&manifest).unwrap(), before);
    finish(controller).await;
}

/// The count the script last showed on Discord.
fn count(discord: &Discord) -> u32 {
    discord
        .details()
        .last()
        .and_then(|shown| shown.as_deref()?.strip_prefix("read ")?.parse().ok())
        .unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread")]
async fn turning_on_a_presence_that_already_runs_does_not_restart_it() {
    let setup = setup();
    // The count lives in the script, so a restart would start it again at 1.
    presence(
        setup.user.path(),
        "count",
        "Count",
        "let n = 0; function read(){ n++; return {n} }          function activity(s){ return {details: 'read ' + s.n} }",
    );
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    eventually("a few reads", || count(&discord) >= 2).await;
    let before = count(&discord);
    tokio::task::block_in_place(|| controller.set_enabled("Count", true)).unwrap();
    // Discord gets at most one update every few seconds, so wait for the next one.
    eventually("the next update", || count(&discord) != before).await;
    assert!(
        count(&discord) > before,
        "the count went back: {:?}",
        discord.details()
    );

    tokio::task::block_in_place(|| controller.set_enabled("Count", false)).unwrap();
    assert!(!controller.presences()[0].running);
    tokio::task::block_in_place(|| controller.set_enabled("Count", true)).unwrap();
    assert!(controller.presences()[0].running);
    assert!(controller.set_enabled("Missing", true).is_err());
    finish(controller).await;
}

fn author_of(folder: &str) -> Option<String> {
    let text = fs::read_to_string(Path::new(folder).join("manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
    manifest["author"].as_str().map(str::to_owned)
}

#[tokio::test(flavor = "multi_thread")]
async fn new_presences_take_the_saved_author_or_else_the_discord_user_name() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Mine"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    // Before anything reached Discord, its user name is not known.
    assert_eq!(controller.author().discord, None);
    assert_eq!(controller.overview().discord_user, None);
    assert!(controller.overview().browsers.is_empty());
    let sessions = controller.extension_sessions();
    assert!(sessions.live.is_empty() && sessions.recent.is_empty());
    eventually("the handshake", || shows(&discord, "Mine")).await;
    eventually("the user name", || controller.author().discord.is_some()).await;
    assert_eq!(controller.author().discord.as_deref(), Some("tester"));
    // The Dashboard shows the same name as the account that receives the activity.
    assert_eq!(
        controller.overview().discord_user.as_deref(),
        Some("tester")
    );

    let web = controller
        .create_presence("From Discord", Template::Web)
        .unwrap();
    assert_eq!(author_of(&web).as_deref(), Some("tester"));

    controller.set_author("  manucabral ").unwrap();
    assert_eq!(controller.author().saved.as_deref(), Some("manucabral"));
    let desktop = controller
        .create_presence("Saved Author", Template::Desktop)
        .unwrap();
    assert_eq!(author_of(&desktop).as_deref(), Some("manucabral"));
    let copy = controller.copy_presence("Mine", "Mine Copy").unwrap();
    assert_eq!(author_of(&copy).as_deref(), Some("manucabral"));

    for bad in ["a\nb", &"x".repeat(65)] {
        assert!(controller.set_author(bad).is_err(), "{bad:?}");
    }
    assert_eq!(controller.author().saved.as_deref(), Some("manucabral"));
    controller.set_author("").unwrap();
    assert_eq!(controller.author().saved, None);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_presence_whose_script_does_not_load_is_not_shown_as_running() {
    let setup = setup();
    presence(setup.user.path(), "broken", "Broken", "function (");
    let controller = start(&setup, &Discord::default()).await;
    let broken = &controller.presences()[0];
    assert!(broken.enabled);
    eventually("the script fails to load", || {
        !controller.presences()[0].running
            && controller
                .overview()
                .status
                .source_errors
                .contains_key("Broken")
    })
    .await;
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn window_choices_are_saved_and_survive_a_restart() {
    let setup = setup();
    let controller = start(&setup, &Discord::default()).await;
    assert!(controller.minimize_to_tray());
    assert!(!controller.startup_asked());
    controller.save_window_choices(false).unwrap();
    finish(controller).await;

    let controller = start(&setup, &Discord::default()).await;
    assert!(!controller.minimize_to_tray());
    assert!(controller.startup_asked());
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_activity_hides_while_the_computer_is_idle_and_returns_after() {
    use std::time::Duration;

    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Mine"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    eventually("the activity", || shows(&discord, "Mine")).await;

    // With the setting off, a long time without input changes nothing.
    controller.update_idle(Some(Duration::from_secs(3 * 3600)));
    let mut preferences = controller.preferences();
    preferences.global.idle_minutes = Some(5);
    // Turning the setting on applies the last reading at once.
    controller.set_preferences(preferences).unwrap();
    eventually("the activity cleared", || {
        discord.details().last() == Some(&None)
    })
    .await;
    eventually("the reason", || {
        controller
            .overview()
            .status
            .rows
            .iter()
            .any(|row| row.reason == rpp_core::policy::IDLE_REASON)
    })
    .await;

    controller.update_idle(Some(Duration::from_secs(2)));
    eventually("the activity back", || shows(&discord, "Mine")).await;

    // An unknown time never hides anything.
    controller.update_idle(None);
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(shows(&discord, "Mine"));

    let mut out_of_range = controller.preferences();
    out_of_range.global.idle_minutes = Some(1);
    assert!(controller.set_preferences(out_of_range).is_err());
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn favorites_are_kept_checked_and_survive_a_restart() {
    let setup = setup();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    controller.set_favorite("YouTube", true).unwrap();
    // Not installed is fine: the catalog lists presences too.
    controller.set_favorite("Not Installed", true).unwrap();
    controller.set_favorite("YouTube", true).unwrap();
    assert_eq!(controller.favorites(), ["Not Installed", "YouTube"]);
    assert!(controller.set_favorite("", true).is_err());
    assert!(controller.set_favorite(&"x".repeat(500), true).is_err());
    controller.set_favorite("Not Installed", false).unwrap();
    finish(controller).await;

    let controller = start(&setup, &discord).await;
    assert_eq!(controller.favorites(), ["YouTube"]);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_diagnostics_report_never_carries_what_a_presence_read() {
    let setup = setup();
    // A script that fails with a page title in its message, as real ones can.
    presence(
        setup.user.path(),
        "leaky",
        "Leaky",
        "function read(){ throw new Error('watching My Secret Title'); } function activity(){}",
    );
    presence(setup.user.path(), "mine", "Mine", &script("Mine"));
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    eventually("the handshake", || shows(&discord, "Mine")).await;
    eventually("the error", || {
        controller
            .diagnostics(None, None)
            .presences
            .iter()
            .any(|p| p.name == "Leaky" && p.error.is_some())
    })
    .await;

    let diagnostics = controller.diagnostics(None, None);
    // The page itself shows the message, on this computer only.
    let leaky = diagnostics
        .presences
        .iter()
        .find(|p| p.name == "Leaky")
        .unwrap();
    assert!(leaky.error.as_deref().unwrap().contains("My Secret Title"));
    assert!(diagnostics.discord_connected);
    // The report says there is an error, never what it says or who is connected.
    assert!(
        diagnostics.report.contains("Leaky 1: error"),
        "{}",
        diagnostics.report
    );
    assert!(
        diagnostics.report.contains("Mine 1: running"),
        "{}",
        diagnostics.report
    );
    assert!(
        !diagnostics.report.contains("Secret"),
        "{}",
        diagnostics.report
    );
    assert!(
        !diagnostics.report.contains("tester"),
        "{}",
        diagnostics.report
    );
    finish(controller).await;
}

fn preset(name: &str, details: &str) -> rpp_config::ManualPreset {
    rpp_config::ManualPreset {
        name: name.into(),
        client_id: "12345678901234567".into(),
        activity: rpp_core::Activity {
            details: Some(details.into()),
            ..rpp_core::Activity::default()
        },
        show_elapsed: false,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn custom_statuses_rotate_until_one_is_chosen_by_hand() {
    let setup = setup();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    for (name, details) in [("A", "First"), ("B", "Second"), ("C", "Third")] {
        controller
            .save_manual_preset(preset(name, details))
            .unwrap();
    }
    assert!(controller.start_rotation(vec!["A".into()], 15).is_err());
    assert!(
        controller
            .start_rotation(vec!["A".into(), "B".into()], 5)
            .is_err()
    );

    controller
        .start_rotation(vec!["A".into(), "B".into(), "C".into()], 15)
        .unwrap();
    eventually("the first status", || shows(&discord, "First")).await;
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    controller.tick_rotation(since + 16);
    eventually("the second status", || shows(&discord, "Second")).await;
    assert_eq!(controller.manual().active.as_deref(), Some("B"));
    controller.tick_rotation(since + 31);
    eventually("the third status", || shows(&discord, "Third")).await;

    // Choosing one by hand ends the rotation.
    controller.show_manual(Some("A")).unwrap();
    assert_eq!(controller.manual().rotation, None);
    eventually("the chosen status", || shows(&discord, "First")).await;
    controller.tick_rotation(since + 46);
    assert_eq!(controller.manual().active.as_deref(), Some("A"));
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_pause_shortcut_is_checked_stored_and_the_toggle_pauses() {
    let setup = setup();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    assert_eq!(controller.pause_shortcut(), None);
    assert!(controller.set_pause_shortcut(Some("P")).is_err());
    assert_eq!(
        controller
            .set_pause_shortcut(Some("alt+ctrl+p"))
            .unwrap()
            .as_deref(),
        Some("Ctrl+Alt+P")
    );
    assert_eq!(controller.pause_shortcut().as_deref(), Some("Ctrl+Alt+P"));
    assert!(controller.toggle_pause().unwrap());
    assert!(controller.preferences().global.paused);
    assert!(!controller.toggle_pause().unwrap());
    controller.set_pause_shortcut(None).unwrap();
    assert_eq!(controller.pause_shortcut(), None);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_presence_goes_with_what_was_saved_about_it() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Hi"));
    let controller = start(&setup, &Discord::default()).await;
    controller.set_enabled("Mine", false).unwrap();
    controller.set_favorite("Mine", true).unwrap();

    controller
        .delete_presence("Mine", |folder| {
            fs::remove_dir_all(folder).map_err(|e| e.to_string())
        })
        .unwrap();
    assert!(controller.presences().is_empty());
    assert!(!setup.user.path().join("mine").exists());

    // A new presence with that name starts clean.
    presence(setup.user.path(), "mine", "Mine", &script("Hi"));
    controller.reload_presences();
    let mine = &controller.presences()[0];
    assert!(mine.enabled);
    assert!(controller.favorites().is_empty());
    controller.shutdown().await;
}

/// Only what the person wrote can go: this is a delete on a path from the library.
#[tokio::test(flavor = "multi_thread")]
async fn a_presence_that_is_not_the_users_cannot_be_deleted() {
    let setup = setup();
    presence(setup.bundled.path(), "theirs", "Theirs", &script("Hi"));
    let controller = start(&setup, &Discord::default()).await;

    let refused = controller.delete_presence("Theirs", |_| panic!("it reached the folder"));
    assert!(refused.is_err());
    assert!(
        controller
            .delete_presence("Nobody", |_| panic!("no folder"))
            .is_err()
    );
    assert_eq!(controller.presences().len(), 1);
    assert!(setup.bundled.path().join("theirs").is_dir());
    controller.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_that_cannot_go_leaves_everything_as_it_was() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Hi"));
    let controller = start(&setup, &Discord::default()).await;
    controller.set_favorite("Mine", true).unwrap();

    let failed = controller.delete_presence("Mine", |_| Err("a program holds a file".to_owned()));
    assert_eq!(failed.unwrap_err(), "a program holds a file");
    assert_eq!(controller.presences().len(), 1);
    assert_eq!(controller.favorites(), ["Mine"]);
    controller.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_the_watched_presence_stops_the_watch() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Hi"));
    let controller = start(&setup, &Discord::default()).await;
    controller.watch(Some("Mine")).unwrap();
    assert_eq!(controller.dev_status().watching.as_deref(), Some("Mine"));

    controller
        .delete_presence("Mine", |folder| {
            fs::remove_dir_all(folder).map_err(|e| e.to_string())
        })
        .unwrap();
    assert_eq!(controller.dev_status().watching, None);
    controller.shutdown().await;
}

/// People delete folders in the file manager, and nothing tells the application.
#[tokio::test(flavor = "multi_thread")]
async fn a_folder_deleted_from_outside_leaves_everywhere() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Hi"));
    presence(setup.user.path(), "other", "Other", &script("Hi"));
    let controller = start(&setup, &Discord::default()).await;
    controller.watch(Some("Mine")).unwrap();
    assert!(!controller.notice_folder_changes(), "nothing moved");

    fs::remove_dir_all(setup.user.path().join("mine")).unwrap();
    assert!(controller.notice_folder_changes());

    let names: Vec<_> = controller.presences().into_iter().map(|p| p.name).collect();
    assert_eq!(names, ["Other"]);
    assert!(controller.inspect("Mine").is_err());
    let status = controller.dev_status();
    assert_eq!(
        (status.watching, status.folder),
        (None, None),
        "the watch ended with the folder"
    );
    assert!(!controller.notice_folder_changes(), "once is enough");
    controller.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_added_from_outside_appears_and_runs() {
    let setup = setup();
    let controller = start(&setup, &Discord::default()).await;
    assert!(controller.presences().is_empty());

    presence(setup.user.path(), "new", "New", &script("Hi"));
    assert!(controller.notice_folder_changes());
    assert_eq!(controller.presences()[0].name, "New");
    controller.shutdown().await;
}

/// A broken save is something to show while the watch goes on, not a reason to end it.
#[tokio::test(flavor = "multi_thread")]
async fn a_broken_manifest_does_not_end_the_watch() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Hi"));
    let controller = start(&setup, &Discord::default()).await;
    controller.watch(Some("Mine")).unwrap();

    fs::write(setup.user.path().join("mine/manifest.json"), "not json").unwrap();
    controller.reload_presences();
    assert!(controller.dev_status().folder.is_some());
    controller.shutdown().await;
}

/// What the application writes itself is already known, so the next look stays cheap.
#[tokio::test(flavor = "multi_thread")]
async fn a_presence_made_in_the_application_is_not_read_twice() {
    let setup = setup();
    let controller = start(&setup, &Discord::default()).await;
    controller.create_presence("Mine", Template::Web).unwrap();
    assert!(!controller.notice_folder_changes());
    controller
        .delete_presence("Mine", |folder| {
            fs::remove_dir_all(folder).map_err(|e| e.to_string())
        })
        .unwrap();
    assert!(!controller.notice_folder_changes());
    controller.shutdown().await;
}

/// A presence written before this existed must get the editor's help too, without asking.
#[tokio::test(flavor = "multi_thread")]
async fn a_presence_of_the_person_gets_what_the_editor_needs() {
    let setup = setup();
    let source = "function read(context) {\n  return {};\n}\n\nfunction activity(seen) {\n  return { details: 'Hello' };\n}\n";
    presence(setup.user.path(), "mine", "Mine", source);
    presence(setup.bundled.path(), "theirs", "Theirs", &script("Hi"));
    let controller = start(&setup, &Discord::default()).await;

    controller.prepare_editors();
    let mine = setup.user.path().join("mine");
    assert!(mine.join("jsconfig.json").is_file());
    assert!(mine.join(".vscode/script-api.d.ts").is_file());
    assert!(mine.join(".vscode/presence-types.d.ts").is_file());
    let source = fs::read_to_string(mine.join("main.js")).unwrap();
    assert!(source.contains("@returns"), "{source}");
    assert!(
        !fs::read_to_string(mine.join("manifest.json"))
            .unwrap()
            .contains("schema")
    );
    assert!(
        !setup.bundled.path().join("theirs/jsconfig.json").exists(),
        "only what the person owns"
    );

    // The comments do not change what the presence does.
    assert_eq!(controller.presences().len(), 2);
    controller.reload_presences();
    assert!(
        controller
            .presences()
            .iter()
            .any(|p| p.name == "Mine" && p.running)
    );

    // A copy does not carry them, and exporting it would not either.
    assert!(
        !fs::read_dir(&mine)
            .unwrap()
            .flatten()
            .all(|entry| entry.file_name() != "jsconfig.json")
    );
    controller.copy_presence("Mine", "Copy").unwrap();
    let names: Vec<_> = controller.presences().into_iter().map(|p| p.name).collect();
    assert!(names.contains(&"Copy".to_owned()));
    controller.shutdown().await;
}

/// A value added to the manifest has to reach the editor while someone works.
#[tokio::test(flavor = "multi_thread")]
async fn the_types_follow_the_manifest_of_a_watched_presence() {
    let setup = setup();
    presence(setup.user.path(), "mine", "Mine", &script("Hi"));
    let controller = start(&setup, &Discord::default()).await;
    controller.prepare_editors();
    let types = setup.user.path().join("mine/.vscode/presence-types.d.ts");
    assert!(
        !fs::read_to_string(&types)
            .unwrap()
            .contains("host: {\n  window")
    );

    controller.watch(Some("Mine")).unwrap();
    let manifest = setup.user.path().join("mine/manifest.json");
    let text = fs::read_to_string(&manifest).unwrap();
    let granted = text.replacen('{', "{\"permissions\":{\"window\":true},", 1);
    fs::write(&manifest, granted).unwrap();
    eventually("the types follow", || {
        fs::read_to_string(&types)
            .unwrap()
            .contains("window: HostWindow;")
    })
    .await;
    controller.shutdown().await;
}
