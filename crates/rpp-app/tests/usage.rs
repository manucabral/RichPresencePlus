//! Counting the time on display, end to end against a fake Discord.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use rpp_app::{Controller, Options};
use rpp_config::ManualPreset;
use rpp_core::Activity;
use rpp_testkit::{Discord, Processes, eventually};

const DAY: &str = "2026-11-05";

fn options(data: &Path, presences: &Path) -> Options {
    let options = Options {
        data_dir: data.to_owned(),
        presence_roots: vec![presences.to_owned()],
        version: "1".into(),
        port_override: Some(0),
        system_language: "en".into(),
        catalog: None,
    };
    rpp_testkit::settings::turn_on_found(&options.data_dir, &options.presence_roots);
    options
}

async fn showing(data: &Path, presences: &Path) -> Controller<Processes> {
    let controller = Controller::start(
        options(data, presences),
        Discord::default(),
        Processes(vec![]),
    )
    .await
    .unwrap();
    let preset = ManualPreset {
        name: "Studying".into(),
        client_id: "123456789012345678".into(),
        activity: Activity {
            details: Some("Reading".into()),
            ..Activity::default()
        },
        show_elapsed: false,
    };
    controller.save_manual_preset(preset).unwrap();
    controller.show_manual(Some("Studying")).unwrap();
    eventually("the status shows", || {
        controller.overview().status.activity.is_some()
    })
    .await;
    controller
}

async fn finish(controller: Controller<Processes>) {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(controller.shutdown());
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn nothing_is_counted_or_written_until_the_person_turns_it_on() {
    let (data, presences) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let controller = showing(data.path(), presences.path()).await;
    let start = Instant::now();
    controller.track_time(Some(DAY), start);
    controller.track_time(Some(DAY), start + Duration::from_secs(5));

    assert!(!controller.usage().on);
    assert!(controller.usage().days.is_empty());
    finish(controller).await;
    assert!(!data.path().join("usage.json").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn time_counts_while_something_shows_and_survives_a_restart() {
    let (data, presences) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let controller = showing(data.path(), presences.path()).await;
    controller.set_track_time(true).unwrap();

    let start = Instant::now();
    controller.track_time(Some(DAY), start);
    controller.track_time(Some(DAY), start + Duration::from_secs(5));
    controller.track_time(Some(DAY), start + Duration::from_secs(10));
    assert_eq!(controller.usage().days[DAY]["Studying"], 10);
    finish(controller).await;

    let again = Controller::start(
        options(data.path(), presences.path()),
        Discord::default(),
        Processes(vec![]),
    )
    .await
    .unwrap();
    let usage = again.usage();
    assert!(usage.on);
    assert_eq!(usage.days[DAY]["Studying"], 10);
    finish(again).await;
}

/// A computer that slept for hours did not show anything to anyone.
#[tokio::test(flavor = "multi_thread")]
async fn a_long_gap_is_not_time_on_display() {
    let (data, presences) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let controller = showing(data.path(), presences.path()).await;
    controller.set_track_time(true).unwrap();

    let start = Instant::now();
    controller.track_time(Some(DAY), start);
    controller.track_time(Some(DAY), start + Duration::from_secs(5));
    controller.track_time(Some(DAY), start + Duration::from_secs(4 * 3600));
    controller.track_time(Some(DAY), start + Duration::from_secs(4 * 3600 + 5));
    assert_eq!(controller.usage().days[DAY]["Studying"], 10);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_leaves_nothing_on_the_disk() {
    let (data, presences) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let controller = showing(data.path(), presences.path()).await;
    controller.set_track_time(true).unwrap();
    let start = Instant::now();
    controller.track_time(Some(DAY), start);
    controller.track_time(Some(DAY), start + Duration::from_secs(5));
    assert!(data.path().join("usage.json").exists());
    assert!(controller.export_usage().contains("Studying"));

    controller.clear_usage().unwrap();
    assert!(controller.usage().days.is_empty());
    assert!(!data.path().join("usage.json").exists());
    finish(controller).await;
    assert!(!data.path().join("usage.json").exists());
}

/// A file that was damaged must not stop the application from starting.
#[tokio::test(flavor = "multi_thread")]
async fn a_damaged_file_counts_as_empty() {
    let (data, presences) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    fs::write(data.path().join("usage.json"), "not json").unwrap();
    let controller = showing(data.path(), presences.path()).await;
    assert!(controller.usage().days.is_empty());
    finish(controller).await;
}
