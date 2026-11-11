//! Installing from the signed catalog through the controller.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use rpp_app::views::{CatalogState, InstallOutcome};
use rpp_app::{Controller, Options};
use rpp_catalog::{Catalog, Channel};
use rpp_testkit::catalog::{Published, served};
use rpp_testkit::{Discord, Processes, eventually};

struct Setup {
    data: tempfile::TempDir,
    user: std::path::PathBuf,
    bundled: tempfile::TempDir,
}

fn setup() -> Setup {
    let data = tempfile::tempdir().unwrap();
    let user = data.path().join("presences");
    Setup {
        data,
        user,
        bundled: tempfile::tempdir().unwrap(),
    }
}

/// A presence in the bundled folder, as the installer ships it.
fn bundle(root: &Path, folder: &str, name: &str, version: &str, details: &str) {
    let directory = root.join(folder);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("manifest.json"),
        format!(
            r#"{{"name":"{name}","version":"{version}","client_id":"12345678901234567","interval":1,"entry":"main.js"}}"#
        ),
    )
    .unwrap();
    fs::write(
        directory.join("main.js"),
        format!(
            "function read(){{return {{}}}} function activity(){{return {{details:'{details}'}}}}"
        ),
    )
    .unwrap();
}

async fn start(
    setup: &Setup,
    published: &Arc<Published>,
    with_keys: bool,
) -> Controller<Processes> {
    let keys = if with_keys {
        Published::keys()
    } else {
        rpp_catalog::Keys::default()
    };
    let source: Box<dyn rpp_catalog::Source> =
        Box::new(rpp_testkit::catalog::Server(published.clone()));
    let catalog = Catalog::new(
        source,
        keys,
        Channel::Stable,
        &setup.user,
        setup.data.path(),
        rpp_catalog::Key::new(&"k".repeat(40)).unwrap(),
    );
    let options = Options {
        data_dir: setup.data.path().to_owned(),
        presence_roots: vec![setup.user.clone(), setup.bundled.path().to_owned()],
        version: "1".into(),
        port_override: Some(0),
        system_language: "en".into(),
        catalog: Some(Arc::new(catalog)),
    };
    rpp_testkit::settings::turn_on_found(&options.data_dir, &options.presence_roots);
    Controller::start(options, Discord::default(), Processes(vec![]))
        .await
        .unwrap()
}

async fn finish(controller: Controller<Processes>) {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(controller.shutdown());
    });
}

fn names(controller: &Controller<Processes>) -> Vec<String> {
    controller.presences().into_iter().map(|p| p.name).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_presence_is_installed_listed_and_runs_without_a_restart() {
    let setup = setup();
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, true).await;

    let view = controller.catalog(true).unwrap();
    assert!(view.available);
    assert_eq!(view.items[0].name, "Alpha");
    assert_eq!(view.items[0].state, CatalogState::Available);
    assert!(view.items[0].size > 0);
    assert!(names(&controller).is_empty());

    tokio::task::block_in_place(|| controller.install_presence("alpha", &[])).unwrap();
    assert_eq!(names(&controller), ["Alpha"]);
    assert!(controller.presences()[0].running);
    assert_eq!(
        controller.catalog(false).unwrap().items[0].state,
        CatalogState::Installed
    );

    tokio::task::block_in_place(|| controller.remove_presence("ALPHA")).unwrap();
    assert!(names(&controller).is_empty());
    assert!(!setup.user.join("alpha").exists());
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_update_replaces_a_bundled_presence_and_removing_it_brings_the_bundled_one_back() {
    let setup = setup();
    bundle(setup.bundled.path(), "alpha", "Alpha", "1.0.0", "Bundled");
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.1.0")]);
    let controller = start(&setup, &published, true).await;
    assert_eq!(controller.presences()[0].version, "1.0.0");

    tokio::task::block_in_place(|| controller.install_presence("Alpha", &[])).unwrap();
    assert_eq!(controller.presences()[0].version, "1.1.0");
    assert_eq!(controller.presences().len(), 1);

    tokio::task::block_in_place(|| controller.remove_presence("Alpha")).unwrap();
    assert_eq!(controller.presences()[0].version, "1.0.0");
    assert!(controller.presences()[0].running);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_installed_copy_changed_by_hand_stops_loading_until_it_is_reinstalled() {
    let setup = setup();
    bundle(setup.bundled.path(), "alpha", "Alpha", "1.0.0", "Bundled");
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.1.0")]);
    let controller = start(&setup, &published, true).await;
    tokio::task::block_in_place(|| controller.install_presence("Alpha", &[])).unwrap();
    assert_eq!(controller.presences()[0].version, "1.1.0");

    fs::write(
        setup.user.join("alpha/main.js"),
        "function read(){ steal() }",
    )
    .unwrap();
    let problems = tokio::task::block_in_place(|| controller.reload_presences());
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0]
            .message
            .contains("changed after it was installed")
    );
    // The bundled copy takes its place, as it does after a removal.
    assert_eq!(controller.presences()[0].version, "1.0.0");
    assert!(controller.presences()[0].running);
    let item = &controller.catalog(false).unwrap().items[0];
    assert_eq!(item.state, CatalogState::Modified);
    assert_eq!(item.installed_version.as_deref(), Some("1.1.0"));

    tokio::task::block_in_place(|| controller.install_presence("Alpha", &[])).unwrap();
    assert_eq!(controller.presences()[0].version, "1.1.0");
    assert_eq!(
        controller.catalog(false).unwrap().items[0].state,
        CatalogState::Installed
    );
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_changed_copy_found_at_start_up_is_not_loaded() {
    let setup = setup();
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, true).await;
    tokio::task::block_in_place(|| controller.install_presence("Alpha", &[])).unwrap();
    finish(controller).await;

    fs::write(setup.user.join("alpha/extra.js"), "x").unwrap();
    let controller = start(&setup, &published, true).await;
    assert!(names(&controller).is_empty());
    assert_eq!(controller.overview().problems.len(), 1);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_presence_the_user_turned_off_stays_off_after_an_update() {
    let setup = setup();
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, true).await;
    tokio::task::block_in_place(|| controller.install_presence("Alpha", &[])).unwrap();
    controller.set_enabled("Alpha", false).unwrap();

    published.publish(200, &[("Alpha", "alpha", "1.1.0")]);
    tokio::task::block_in_place(|| {
        controller.catalog(true).unwrap();
        controller.install_presence("Alpha", &[]).unwrap();
    });
    let alpha = &controller.presences()[0];
    assert_eq!(alpha.version, "1.1.0");
    assert!(!alpha.enabled && !alpha.running);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tampered_download_is_refused_and_the_installed_copy_keeps_running() {
    let setup = setup();
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, true).await;
    tokio::task::block_in_place(|| controller.install_presence("Alpha", &[])).unwrap();

    published.publish(200, &[("Alpha", "alpha", "1.1.0")]);
    published.tamper("alpha", "main.js", b"function read(){ steal() }");
    let result = tokio::task::block_in_place(|| {
        controller.catalog(true).unwrap();
        controller.install_presence("Alpha", &[])
    });
    assert!(result.unwrap_err().contains("does not match"));
    assert_eq!(controller.presences()[0].version, "1.0.0");
    assert!(controller.presences()[0].running);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn installs_are_counted_by_name_and_missing_counts_are_not_an_error() {
    let setup = setup();
    let (published, _source) = served();
    published.publish(
        100,
        &[("Alpha", "alpha", "1.0.0"), ("Beta", "beta", "1.0.0")],
    );
    let controller = start(&setup, &published, true).await;
    // Nothing published yet: the list still works and there are no numbers.
    assert_eq!(controller.catalog(true).unwrap().items.len(), 2);
    assert!(controller.catalog_installs().is_empty());
    finish(controller).await;

    published.count_installs(&[("alpha", 1234), ("gone", 9)]);
    let controller = start(&setup, &published, true).await;
    let installs = controller.catalog_installs();
    assert_eq!(installs.len(), 1);
    assert_eq!(installs["Alpha"], 1234);
    finish(controller).await;

    // A build without keys asks for nothing.
    let controller = start(&setup, &published, false).await;
    assert!(controller.catalog_installs().is_empty());
    assert_eq!(published.index_downloads(), 2);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn without_keys_the_catalog_reports_itself_unavailable_and_installs_nothing() {
    let setup = setup();
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, false).await;
    let view = controller.catalog(true).unwrap();
    assert!(!view.available && view.items.is_empty());
    assert!(controller.install_presence("Alpha", &[]).is_err());
    assert_eq!(published.index_downloads(), 0);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn hand_made_presences_cannot_be_removed_through_the_catalog() {
    let setup = setup();
    bundle(&setup.user, "mine", "Mine", "1.0.0", "Mine");
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, true).await;
    let error = tokio::task::block_in_place(|| controller.remove_presence("Mine")).unwrap_err();
    assert!(error.contains("not installed from the catalog"));
    assert_eq!(names(&controller), ["Mine"]);
    assert!(setup.user.join("mine").is_dir());
    eventually("the presence keeps running", || {
        controller.presences()[0].running
    })
    .await;
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_offline_catalog_is_an_error_message_not_a_crash() {
    let setup = setup();
    let (published, _source) = served();
    let controller = start(&setup, &published, true).await;
    assert!(
        controller
            .catalog(true)
            .unwrap_err()
            .contains("cannot reach")
    );
    published.go_offline();
    assert!(controller.install_presence("Alpha", &[]).is_err());
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_update_can_be_undone_and_removing_forgets_the_history() {
    let setup = setup();
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, true).await;
    tokio::task::block_in_place(|| controller.install_presence("Alpha", &[])).unwrap();
    assert!(controller.presence_history("Alpha").unwrap().is_empty());

    published.publish(200, &[("Alpha", "alpha", "1.1.0")]);
    tokio::task::block_in_place(|| {
        controller.catalog(true).unwrap();
        controller.install_presence("Alpha", &[]).unwrap();
    });
    assert_eq!(controller.presences()[0].version, "1.1.0");
    assert_eq!(controller.presence_history("Alpha").unwrap(), ["1.0.0"]);
    let listed = controller.catalog(false).unwrap();
    assert_eq!(listed.items[0].history, ["1.0.0"]);

    tokio::task::block_in_place(|| controller.restore_presence("Alpha", "1.0.0")).unwrap();
    assert_eq!(controller.presences()[0].version, "1.0.0");
    assert!(controller.presences()[0].running);
    assert_eq!(controller.presence_history("Alpha").unwrap(), ["1.1.0"]);

    assert!(controller.restore_presence("Alpha", "9.9.9").is_err());
    assert!(
        controller
            .restore_presence("Alpha", "not a version")
            .is_err()
    );
    assert!(controller.restore_presence("Nothing", "1.0.0").is_err());
    assert_eq!(controller.presences()[0].version, "1.0.0");

    tokio::task::block_in_place(|| controller.remove_presence("Alpha")).unwrap();
    published.publish(300, &[("Alpha", "alpha", "1.0.0")]);
    tokio::task::block_in_place(|| {
        controller.catalog(true).unwrap();
        controller.install_presence("Alpha", &[]).unwrap();
    });
    assert!(
        controller.presence_history("Alpha").unwrap().is_empty(),
        "a removed presence leaves no history behind"
    );
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_catalog_presence_is_not_editable_and_its_files_stay_as_installed() {
    let setup = setup();
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = Arc::new(start(&setup, &published, true).await);
    tokio::task::block_in_place(|| controller.install_presence("Alpha", &[])).unwrap();

    let view = &controller.presences()[0];
    assert_eq!(view.origin, rpp_app::Origin::Catalog);
    assert!(!view.editable);
    let manifest = setup.user.join("alpha/manifest.json");
    let before = fs::read_to_string(&manifest).unwrap();
    let changes = rpp_app::ManifestChanges {
        author: Some("me".into()),
        version: Some("9.9.9".into()),
        ..Default::default()
    };
    let error = controller.update_manifest("Alpha", &changes).unwrap_err();
    assert!(error.contains("Only presences you wrote"), "{error}");
    let error = controller.watch(Some("Alpha")).unwrap_err();
    assert!(error.contains("Only presences you wrote"), "{error}");
    assert_eq!(fs::read_to_string(&manifest).unwrap(), before);
    assert_eq!(
        controller.catalog(false).unwrap().items[0].state,
        CatalogState::Installed,
        "still intact after the refused edits"
    );
    // A copy is the user's own and can be edited.
    tokio::task::block_in_place(|| controller.copy_presence("Alpha", "Alpha Fork")).unwrap();
    let fork = controller
        .presences()
        .into_iter()
        .find(|p| p.name == "Alpha Fork")
        .unwrap();
    assert!(fork.editable && fork.origin == rpp_app::Origin::Local);
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(controller.shutdown());
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn a_catalog_install_is_refused_when_a_presence_of_the_user_has_the_name() {
    let setup = setup();
    bundle(&setup.user, "mine", "Alpha", "1.0.0", "Mine");
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, true).await;

    for name in ["Alpha", "alpha", "ALPHA"] {
        let error =
            tokio::task::block_in_place(|| controller.install_presence(name, &[])).unwrap_err();
        assert!(error.contains("already called"), "{name}: {error}");
    }
    assert!(!setup.user.join("alpha").exists());
    assert_eq!(controller.presences()[0].origin, rpp_app::Origin::Local);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_forged_catalog_marker_does_not_make_a_folder_pass_for_an_official_one() {
    let setup = setup();
    bundle(setup.bundled.path(), "alpha", "Alpha", "1.0.0", "Bundled");
    // A folder dropped in by hand that claims the catalog installed it.
    bundle(&setup.user, "alpha", "Alpha", "9.9.9", "Forged");
    fs::write(
        setup.user.join("alpha/.catalog.json"),
        r#"{"name":"Alpha","folder":"alpha","version":"9.9.9","commit":"0123456789abcdef0123456789abcdef01234567","files":{},"seal":"00"}"#,
    )
    .unwrap();
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, true).await;

    let list = controller.presences();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].version, "1.0.0", "the bundled copy runs");
    assert_eq!(list[0].origin, rpp_app::Origin::Bundled);
    assert_eq!(controller.overview().problems.len(), 1);
    assert_eq!(
        controller.catalog(false).unwrap().items[0].state,
        CatalogState::Modified
    );
    finish(controller).await;
}

/// An update that reaches for more waits for a yes, and the old copy keeps working.
#[tokio::test(flavor = "multi_thread")]
async fn an_update_that_asks_for_more_waits_for_a_yes() {
    let setup = setup();
    let (published, _source) = served();
    published.publish(100, &[("Alpha", "alpha", "1.0.0")]);
    let controller = start(&setup, &published, true).await;
    tokio::task::block_in_place(|| controller.install_presence("Alpha", &[])).unwrap();

    published.publish_asking(200, &[("Alpha", "alpha", "1.1.0")], r#"{"process":true}"#);
    let outcome = tokio::task::block_in_place(|| {
        controller.catalog(true).unwrap();
        controller.install_presence("Alpha", &[])
    })
    .unwrap();
    let InstallOutcome::Asks { access, grants } = outcome else {
        panic!("the update went through");
    };
    assert!(access.process);
    assert_eq!(controller.presences()[0].version, "1.0.0");
    assert!(controller.presences()[0].running);
    assert!(!controller.presences()[0].access.process);

    let outcome = tokio::task::block_in_place(|| controller.install_presence("Alpha", &grants));
    assert_eq!(outcome.unwrap(), InstallOutcome::Installed);
    assert_eq!(controller.presences()[0].version, "1.1.0");
    assert!(controller.presences()[0].access.process);
    finish(controller).await;
}
