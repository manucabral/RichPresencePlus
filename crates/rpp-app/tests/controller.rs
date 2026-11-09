//! The controller against a fake Discord, real settings files and real presence folders.

use std::fs;
use std::net::TcpListener;
use std::path::Path;

use rpp_app::{Controller, Options};
use rpp_core::Preferences;
use rpp_testkit::{CountedProcesses, Discord, Processes, eventually};

const SCRIPT: &str =
    "function read(){return {}} function activity(){return {details:'Hello there'}}";

fn presence(root: &Path, folder: &str, name: &str) {
    let directory = root.join(folder);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("manifest.json"),
        format!(
            r#"{{"name":"{name}","version":"1.2","description":"A test presence","client_id":"12345678901234567",
                "kind":"game","interval":1,"permissions":{{"process":true}},
                "page":{{"matches":["*://a.io/*"]}}}}"#
        ),
    )
    .unwrap();
    fs::write(directory.join("main.js"), SCRIPT).unwrap();
}

struct Setup {
    data: tempfile::TempDir,
    presences: tempfile::TempDir,
}

fn setup() -> Setup {
    let setup = Setup {
        data: tempfile::tempdir().unwrap(),
        presences: tempfile::tempdir().unwrap(),
    };
    presence(setup.presences.path(), "demo", "Demo");
    setup
}

fn options(setup: &Setup) -> Options {
    Options {
        data_dir: setup.data.path().to_owned(),
        presence_roots: vec![setup.presences.path().to_owned()],
        version: "9.9.9".into(),
        port_override: Some(0),
        system_language: "en".into(),
        catalog: None,
    }
}

async fn start(setup: &Setup, discord: &Discord) -> Controller<Processes> {
    let host = Processes(vec!["explorer.exe".into(), "Discord.exe".into()]);
    Controller::start(options(setup), discord.clone(), host)
        .await
        .unwrap()
}

async fn finish(controller: Controller<Processes>) {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(controller.shutdown());
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn it_lists_presences_publishes_and_describes_their_access() {
    let setup = setup();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;

    let presences = controller.presences();
    assert_eq!(presences.len(), 1);
    let demo = &presences[0];
    assert!(demo.enabled && demo.running);
    assert_eq!(demo.version, "1.2");
    assert!(demo.access.process && !demo.access.window);
    assert_eq!(demo.access.sites, ["*://a.io/*"]);

    eventually("the activity", || {
        discord.details() == [Some("Hello there".into())]
    })
    .await;
    let overview = controller.overview();
    assert_eq!(overview.version, "9.9.9");
    assert!(overview.discord_running);
    assert_eq!(overview.extensions, 0);
    assert_eq!(overview.warning, None);
    assert_eq!(overview.status.winner.as_deref(), Some("Demo"));
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn turning_a_presence_off_stops_it_and_the_choice_is_remembered() {
    let setup = setup();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    eventually("the activity", || !discord.details().is_empty()).await;

    controller.set_enabled("Demo", false).unwrap();
    assert!(!controller.presences()[0].enabled && !controller.presences()[0].running);
    eventually("the clear", || discord.details().last() == Some(&None)).await;
    finish(controller).await;

    let quiet = Discord::default();
    let again = start(&setup, &quiet).await;
    assert!(!again.presences()[0].enabled && !again.presences()[0].running);
    again.set_enabled("Demo", true).unwrap();
    assert!(again.presences()[0].running);
    eventually("the activity again", || {
        quiet.details() == [Some("Hello there".into())]
    })
    .await;
    finish(again).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_presences_are_refused() {
    let setup = setup();
    let controller = start(&setup, &Discord::default()).await;
    assert!(
        controller
            .set_enabled("Nope", true)
            .unwrap_err()
            .contains("Unknown")
    );
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn preferences_are_validated_saved_and_applied() {
    let setup = setup();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    eventually("the activity", || !discord.details().is_empty()).await;

    let mut invalid = Preferences::default();
    invalid.global.excluded_domains = vec!["https://not-a-host/".into()];
    assert!(controller.set_preferences(invalid).is_err());
    assert_eq!(controller.preferences(), Preferences::default());

    let mut paused = Preferences::default();
    paused.global.paused = true;
    paused.global.excluded_domains = vec!["  Example.COM ".into()];
    let stored = controller.set_preferences(paused).unwrap();
    assert_eq!(stored.global.excluded_domains, ["example.com"]);
    assert!(controller.overview().paused);
    eventually("the clear", || discord.details().last() == Some(&None)).await;
    finish(controller).await;

    let again = start(&setup, &Discord::default()).await;
    assert!(again.preferences().global.paused);
    finish(again).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_pairing_code_is_stable_across_restarts() {
    let setup = setup();
    let first = start(&setup, &Discord::default()).await;
    let one = first.pairing();
    assert_eq!(one.code, format!("{}:{}", one.port, one.token));
    assert_eq!(one.token.len(), 64);
    finish(first).await;

    let second = start(&setup, &Discord::default()).await;
    assert_eq!(second.pairing().token, one.token);
    finish(second).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_damaged_settings_file_starts_paused_with_a_warning() {
    let setup = setup();
    fs::write(setup.data.path().join("settings.json"), "{ broken").unwrap();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    let overview = controller.overview();
    assert!(overview.paused);
    assert!(overview.warning.unwrap().contains("settings.invalid.json"));
    tokio::time::sleep(std::time::Duration::from_millis(1800)).await;
    assert!(
        discord.details().is_empty(),
        "nothing may be published while paused"
    );
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_busy_bridge_port_falls_back_to_a_free_one_and_says_so() {
    let setup = setup();
    let taken = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();
    let settings = rpp_config::Settings {
        bridge_port: port,
        ..Default::default()
    };
    rpp_config::SettingsStore::new(setup.data.path())
        .save(&settings)
        .unwrap();
    let options = Options {
        data_dir: setup.data.path().to_owned(),
        presence_roots: vec![setup.presences.path().to_owned()],
        version: "1".into(),
        port_override: None,
        system_language: "en".into(),
        catalog: None,
    };
    let controller = Controller::start(options, Discord::default(), Processes(vec![]))
        .await
        .unwrap();
    let pairing = controller.pairing();
    assert_ne!(pairing.port, port);
    assert!(
        controller
            .overview()
            .warning
            .unwrap()
            .contains("paste the new one")
    );
    // The extension learns the port actually used, not the saved one.
    assert_eq!(
        rpp_bridge::native::load_pairing(setup.data.path()),
        Some(rpp_bridge::native::Pairing {
            port: pairing.port,
            token: pairing.token,
        })
    );
    assert!(!controller.overview().discord_running);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn presence_folders_that_fail_are_reported_not_fatal() {
    let setup = setup();
    fs::create_dir(setup.presences.path().join("broken")).unwrap();
    fs::write(setup.presences.path().join("broken/manifest.json"), "{").unwrap();
    let controller = start(&setup, &Discord::default()).await;
    assert_eq!(controller.presences().len(), 1);
    assert_eq!(controller.overview().problems[0].folder, "broken");
    finish(controller).await;
}

const TRANSLATED: &str =
    "function read(){return {}} function activity(){return {details:t('greeting',{who:'Ana'})}}";

fn translated_presence(root: &Path) {
    presence(root, "words", "Words");
    let directory = root.join("words");
    fs::write(directory.join("main.js"), TRANSLATED).unwrap();
    fs::create_dir_all(directory.join("locales")).unwrap();
    fs::write(
        directory.join("locales/en.json"),
        r#"{"greeting":"Hello {who}","description":"Says hello"}"#,
    )
    .unwrap();
    fs::write(
        directory.join("locales/es.json"),
        r#"{"greeting":"Hola {who}","description":"Saluda"}"#,
    )
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_language_changes_presence_texts_and_is_remembered() {
    let setup = setup();
    translated_presence(setup.presences.path());
    fs::remove_dir_all(setup.presences.path().join("demo")).unwrap();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    assert_eq!(controller.language().setting, "auto");
    assert_eq!(
        controller.presences()[0].description.as_deref(),
        Some("Says hello")
    );
    eventually("the english text", || {
        discord.details() == [Some("Hello Ana".into())]
    })
    .await;

    controller.set_language("es-AR").unwrap();
    assert_eq!(controller.language().setting, "es-AR");
    assert_eq!(
        controller.presences()[0].description.as_deref(),
        Some("Saluda")
    );
    eventually("the spanish text", || {
        discord.details().last() == Some(&Some("Hola Ana".into()))
    })
    .await;
    assert!(controller.set_language("../x").is_err());
    assert_eq!(controller.language().setting, "es-AR");
    finish(controller).await;

    let again = start(&setup, &Discord::default()).await;
    assert_eq!(again.language().setting, "es-AR");
    assert_eq!(again.presences()[0].description.as_deref(), Some("Saluda"));
    finish(again).await;
}

const OPTIONED: &str = "function read(c){return {who: c.options.who, loud: c.options.loud}} \
     function activity(s){return {details: (s.loud ? 'HELLO ' : 'Hello ') + s.who}}";

fn optioned_presence(root: &Path) {
    let directory = root.join("options");
    fs::create_dir_all(directory.join("locales")).unwrap();
    fs::write(
        directory.join("manifest.json"),
        r#"{"name":"Options","version":"1","client_id":"12345678901234567","interval":1,
            "options":[
              {"type":"text","key":"who","default":"Ana","max_length":10},
              {"type":"boolean","key":"loud","default":false},
              {"type":"choice","key":"style","default":"a","choices":["a","b"]}
            ]}"#,
    )
    .unwrap();
    fs::write(directory.join("main.js"), OPTIONED).unwrap();
    fs::write(
        directory.join("locales/en.json"),
        r#"{"option.who":"Name to greet","option.style.b":"Second style","help.who":"Who the card greets"}"#,
    )
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn options_are_listed_saved_and_reach_the_running_script() {
    let setup = setup();
    fs::remove_dir_all(setup.presences.path().join("demo")).unwrap();
    optioned_presence(setup.presences.path());
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;

    let options = &controller.presences()[0].options;
    assert_eq!(options.len(), 3);
    assert_eq!(options[0].label, "Name to greet");
    assert_eq!(options[0].value, serde_json::json!("Ana"));
    // A label the presence does not translate falls back to the key.
    assert_eq!(options[1].label, "loud");
    assert_eq!(options[2].choice_labels, ["a", "Second style"]);
    assert_eq!(options[0].help.as_deref(), Some("Who the card greets"));
    assert_eq!(options[1].help, None);
    eventually("the default text", || {
        discord.details().last() == Some(&Some("Hello Ana".into()))
    })
    .await;

    controller
        .set_option("Options", "who", serde_json::json!("Bo"))
        .unwrap();
    controller
        .set_option("Options", "loud", serde_json::json!(true))
        .unwrap();
    eventually("the new options, without a restart", || {
        discord.details().last() == Some(&Some("HELLO Bo".into()))
    })
    .await;

    // Values that do not fit, and options or presences that do not exist.
    for (key, value) in [
        ("who", serde_json::json!("far too long a name")),
        ("loud", serde_json::json!("yes")),
        ("style", serde_json::json!("c")),
        ("missing", serde_json::json!(1)),
    ] {
        assert!(
            controller.set_option("Options", key, value).is_err(),
            "{key}"
        );
    }
    assert!(
        controller
            .set_option("Nope", "who", serde_json::json!("x"))
            .is_err()
    );
    assert_eq!(
        controller.presences()[0].options[0].value,
        serde_json::json!("Bo")
    );
    finish(controller).await;

    // The choice survives a restart.
    let again = Discord::default();
    let restarted = start(&setup, &again).await;
    assert_eq!(
        restarted.presences()[0].options[1].value,
        serde_json::json!(true)
    );
    eventually("the saved options", || {
        again.details().last() == Some(&Some("HELLO Bo".into()))
    })
    .await;
    finish(restarted).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_sites_a_person_adds_are_read_and_remembered() {
    let setup = setup();
    let directory = setup.presences.path().join("server");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("manifest.json"),
        r#"{"name":"Server","version":"1","client_id":"12345678901234567","interval":1,
            "page":{"user_sites":true,"extract":{"t":{"from":"response","url":"/api/now"}}}}"#,
    )
    .unwrap();
    fs::write(directory.join("main.js"), SCRIPT).unwrap();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    let server = |controller: &Controller<Processes>| {
        controller
            .presences()
            .into_iter()
            .find(|p| p.name == "Server")
            .unwrap()
    };
    assert_eq!(server(&controller).user_sites, Some(vec![]));
    // A presence with sites of its own lists none until the person adds one.
    let demo = controller
        .presences()
        .into_iter()
        .find(|p| p.name == "Demo")
        .unwrap();
    assert_eq!(demo.user_sites, None);

    let saved = controller
        .set_sites(
            "Server",
            &[
                "http://Media.Home:8096/web".into(),
                "media.home".into(),
                "192.168.1.5".into(),
            ],
        )
        .unwrap();
    assert_eq!(saved, ["media.home", "192.168.1.5"]);
    let view = server(&controller);
    assert_eq!(view.user_sites.as_deref(), Some(&saved[..]));
    assert_eq!(view.access.sites, ["*://media.home/*", "*://192.168.1.5/*"]);

    assert!(
        controller
            .set_sites("Server", &["user@host".into()])
            .is_err()
    );
    assert!(controller.set_sites("Demo", &["a.io".into()]).is_err());
    assert!(controller.set_sites("Nope", &["a.io".into()]).is_err());
    let nine: Vec<String> = (0..9).map(|n| format!("h{n}.lan")).collect();
    assert!(controller.set_sites("Server", &nine).is_err());
    assert_eq!(
        server(&controller).user_sites.as_deref(),
        Some(&saved[..]),
        "a refused change keeps the sites"
    );
    finish(controller).await;

    let controller = start(&setup, &discord).await;
    assert_eq!(
        server(&controller).user_sites.as_deref(),
        Some(&saved[..]),
        "the sites are remembered"
    );
    controller.set_sites("Server", &[]).unwrap();
    assert_eq!(server(&controller).user_sites, Some(vec![]));
    finish(controller).await;
}

fn written(name: &str, details: &str) -> rpp_config::ManualPreset {
    rpp_config::ManualPreset {
        name: name.into(),
        client_id: "12345678901234567".into(),
        activity: rpp_core::Activity {
            details: Some(details.into()),
            ..Default::default()
        },
        show_elapsed: false,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manual_preset_outranks_scripts_and_hands_back_when_stopped() {
    let setup = setup();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    eventually("the script activity", || {
        discord.details().last() == Some(&Some("Hello there".into()))
    })
    .await;

    controller
        .save_manual_preset(written("Studying", "Reading a book"))
        .unwrap();
    assert!(
        controller.manual().active.is_none(),
        "saving does not show it"
    );
    controller.show_manual(Some("Studying")).unwrap();
    eventually("the manual activity", || {
        discord.details().last() == Some(&Some("Reading a book".into()))
    })
    .await;
    // The winner is the source; the row carries the name the user gave.
    let status = controller.overview().status;
    assert_eq!(status.winner.as_deref(), Some(".manual"));
    let row = status.rows.iter().find(|row| row.selected).unwrap();
    assert_eq!(row.label, "Studying");
    assert_eq!(row.kind, rpp_core::Kind::Manual);

    controller.show_manual(None).unwrap();
    eventually("the script again", || {
        discord.details().last() == Some(&Some("Hello there".into()))
    })
    .await;
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn editing_the_preset_on_display_changes_it_at_once() {
    let setup = setup();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    controller
        .save_manual_preset(written("Studying", "First"))
        .unwrap();
    controller.show_manual(Some("Studying")).unwrap();
    eventually("the first text", || {
        discord.details().last() == Some(&Some("First".into()))
    })
    .await;
    controller
        .save_manual_preset(written("Studying", "Second"))
        .unwrap();
    eventually("the second text", || {
        discord.details().last() == Some(&Some("Second".into()))
    })
    .await;
    assert_eq!(controller.manual().presets.len(), 1);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_presets_are_refused_and_nothing_is_saved() {
    let setup = setup();
    let controller = start(&setup, &Discord::default()).await;
    let mut no_id = written("A", "xx");
    no_id.client_id = "abc".into();
    let mut blank = written("B", "xx");
    blank.activity.details = None;
    for bad in [no_id, blank, written("   ", "xx")] {
        assert!(controller.save_manual_preset(bad).is_err());
    }
    for number in 0..rpp_config::MAX_PRESETS {
        controller
            .save_manual_preset(written(&format!("P{number}"), "xx"))
            .unwrap();
    }
    assert!(
        controller
            .save_manual_preset(written("one more", "xx"))
            .is_err()
    );
    assert_eq!(controller.manual().presets.len(), rpp_config::MAX_PRESETS);
    assert!(controller.show_manual(Some("missing")).is_err());
    assert!(controller.delete_manual_preset("missing").is_err());
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_preset_on_display_comes_back_after_a_restart_and_deleting_withdraws_it() {
    let setup = setup();
    let first = Discord::default();
    let controller = start(&setup, &first).await;
    controller
        .save_manual_preset(written("Studying", "Reading"))
        .unwrap();
    controller.show_manual(Some("Studying")).unwrap();
    eventually("shown", || {
        first.details().last() == Some(&Some("Reading".into()))
    })
    .await;
    finish(controller).await;

    let second = Discord::default();
    let restarted = start(&setup, &second).await;
    assert_eq!(restarted.manual().active.as_deref(), Some("Studying"));
    eventually("shown again at start", || {
        second.details().last() == Some(&Some("Reading".into()))
    })
    .await;

    restarted.delete_manual_preset("Studying").unwrap();
    assert!(restarted.manual().presets.is_empty());
    assert!(restarted.manual().active.is_none());
    eventually("withdrawn", || {
        second.details().last() == Some(&Some("Hello there".into()))
    })
    .await;
    finish(restarted).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_profile_restores_the_rules_and_the_presences_that_were_on() {
    let setup = setup();
    let discord = Discord::default();
    let controller = start(&setup, &discord).await;
    eventually("the script activity", || {
        discord.details().last() == Some(&Some("Hello there".into()))
    })
    .await;
    controller.save_profile("Streaming").unwrap();

    // A different state: the presence off and publishing paused.
    controller.set_enabled("Demo", false).unwrap();
    let mut quiet = controller.preferences();
    quiet.global.paused = true;
    controller.set_preferences(quiet).unwrap();
    controller.save_profile("Quiet").unwrap();
    assert_eq!(controller.profiles().len(), 2);

    controller.apply_profile("Streaming").unwrap();
    assert!(!controller.preferences().global.paused);
    assert!(controller.presences()[0].enabled && controller.presences()[0].running);
    eventually("the activity is back", || {
        discord.details().last() == Some(&Some("Hello there".into()))
    })
    .await;

    controller.apply_profile("Quiet").unwrap();
    assert!(controller.preferences().global.paused);
    assert!(!controller.presences()[0].enabled && !controller.presences()[0].running);
    finish(controller).await;

    // Profiles and the current choice survive a restart.
    let restarted = start(&setup, &Discord::default()).await;
    assert_eq!(restarted.profiles().len(), 2);
    assert!(restarted.preferences().global.paused);
    finish(restarted).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn profile_names_and_lists_are_checked() {
    let setup = setup();
    let controller = start(&setup, &Discord::default()).await;
    assert!(controller.save_profile("   ").is_err());
    assert!(controller.apply_profile("Missing").is_err());
    assert!(controller.delete_profile("Missing").is_err());
    controller.save_profile("One").unwrap();
    controller.save_profile("One").unwrap();
    assert_eq!(controller.profiles().len(), 1, "same name replaces");
    controller.delete_profile("One").unwrap();
    assert!(controller.profiles().is_empty());
    for number in 0..rpp_config::MAX_PROFILES {
        controller.save_profile(&format!("P{number}")).unwrap();
    }
    assert!(controller.save_profile("one more").is_err());
    finish(controller).await;
}

fn with_metadata(root: &Path, folder: &str, name: &str, icon: &str) {
    let directory = root.join(folder);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("manifest.json"),
        format!(
            r##"{{"name":"{name}","version":"1","client_id":"12345678901234567",
                "author":"Ana","color":"#A56786","tags":["video","music"],"icon":"{icon}"}}"##
        ),
    )
    .unwrap();
    fs::write(directory.join("main.js"), SCRIPT).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn authors_colors_tags_and_logos_reach_the_window() {
    let setup = setup();
    fs::remove_dir_all(setup.presences.path().join("demo")).unwrap();
    let root = setup.presences.path();
    with_metadata(root, "local", "Local", "logo.png");
    fs::write(root.join("local/logo.png"), b"not really a picture").unwrap();
    with_metadata(root, "remote", "Remote", "https://img.example.com/logo.png");
    with_metadata(root, "huge", "Huge", "big.webp");
    fs::write(root.join("huge/big.webp"), vec![0_u8; 300 * 1024]).unwrap();
    with_metadata(root, "absent", "Absent", "nothing.png");

    let controller = start(&setup, &Discord::default()).await;
    let views = controller.presences();
    let by_name = |name: &str| views.iter().find(|view| view.name == name).unwrap();
    let local = by_name("Local");
    assert_eq!(local.author.as_deref(), Some("Ana"));
    assert_eq!(local.color.as_deref(), Some("#A56786"));
    assert_eq!(local.tags, ["video", "music"]);
    // A logo kept as a file is not sent with every list: it has its own call.
    assert_eq!(local.icon, None);
    assert_eq!(
        by_name("Remote").icon.as_deref(),
        Some("https://img.example.com/logo.png")
    );

    let icons = controller.icons();
    assert!(icons["Local"].starts_with("data:image/png;base64,"));
    assert!(!icons.contains_key("Remote"), "an address needs no file");
    assert!(
        !icons.contains_key("Huge"),
        "a logo over 256 KiB is left out"
    );
    assert!(!icons.contains_key("Absent"));
    finish(controller).await;
}

/// The tray asks all day with nobody looking, and each list of processes costs
/// about 15 ms of a core.
#[tokio::test(flavor = "multi_thread")]
async fn the_tray_does_not_list_every_process_each_time_it_looks() {
    let setup = setup();
    let host = CountedProcesses::new(&["explorer.exe", "Discord.exe"]);
    let controller = Controller::start(options(&setup), Discord::default(), host.clone())
        .await
        .unwrap();

    let glance = controller.glance();
    assert!(glance.discord_running);
    assert_eq!(glance.language, "en");
    assert!(!glance.paused);
    let first = host.lists();

    // Looking again after the window's own limit of 2 seconds is what the tray does
    // all day. It must not list the processes again.
    std::thread::sleep(std::time::Duration::from_millis(2100));
    for _ in 0..3 {
        assert!(controller.glance().discord_running);
    }
    assert_eq!(
        host.lists(),
        first,
        "the tray listed the processes again within 30 seconds"
    );

    // The window asks for a fresher answer, and gets it.
    assert!(controller.overview().discord_running);
    assert_eq!(host.lists(), first + 1);

    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(controller.shutdown());
    });
}

/// Listing the programs is what the rule costs, so it runs only for those who use it.
#[tokio::test(flavor = "multi_thread")]
async fn the_quiet_rule_lists_programs_only_for_who_has_one_and_not_each_time() {
    let setup = setup();
    let host = CountedProcesses::new(&["explorer.exe", "OBS64.exe"]);
    let controller = Controller::start(options(&setup), Discord::default(), host.clone())
        .await
        .unwrap();

    let before = host.lists();
    for _ in 0..50 {
        controller.update_quiet(Some(600));
    }
    assert_eq!(host.lists(), before, "nobody asked for this rule");

    let mut preferences = controller.preferences();
    preferences.global.quiet_programs = vec!["obs64.exe".into()];
    controller.set_preferences(preferences).unwrap();
    for _ in 0..50 {
        controller.update_quiet(Some(600));
    }
    assert_eq!(host.lists(), before + 1, "one list every 15 seconds");

    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(controller.shutdown());
    });
}
