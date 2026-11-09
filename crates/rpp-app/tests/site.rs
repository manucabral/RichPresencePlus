//! A presence asked for from a page, from the extension's message to a running presence.

use std::net::SocketAddr;

use rpp_app::{Controller, Options};
use rpp_testkit::extension::Extension;
use rpp_testkit::{Discord, Processes, eventually};
use serde_json::json;

async fn asked(message: serde_json::Value) -> (Controller<Processes>, tempfile::TempDir) {
    let (data, presences) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let options = Options {
        data_dir: data.path().to_owned(),
        presence_roots: vec![presences.path().to_owned()],
        version: "1".into(),
        port_override: Some(0),
        system_language: "en".into(),
        catalog: None,
    };
    rpp_testkit::settings::turn_on_found(&options.data_dir, &options.presence_roots);
    let controller = Controller::start(options, Discord::default(), Processes(vec![]))
        .await
        .unwrap();
    let pairing = controller.pairing();
    let address: SocketAddr = format!("127.0.0.1:{}", pairing.port).parse().unwrap();
    let mut extension = Extension::pair(address, &pairing.token).await;
    extension.send(message).await;
    // The answer to a ping says the message before it was read.
    extension.send(json!({"type": "ping"})).await;
    extension.receive().await;
    (controller, presences)
}

async fn finish(controller: Controller<Processes>) {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(controller.shutdown());
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pick_on_a_page_becomes_a_presence_that_reads_only_that_site() {
    let (controller, _folder) = asked(json!({
        "type": "make_site", "name": "Blog", "host": "blog.example.com",
        "first": "h1", "second": "article > h2",
    }))
    .await;
    eventually("the request arrives", || {
        controller.make_requested_sites();
        controller.presences().iter().any(|p| p.name == "Blog")
    })
    .await;

    let blog = &controller.presences()[0];
    assert!(blog.editable, "it is the person's to change");
    assert_eq!(blog.access.sites, ["*://blog.example.com/*"]);
    assert!(blog.access.http.is_empty() && !blog.access.process && !blog.access.window);
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bad_request_makes_nothing_and_breaks_nothing() {
    let (controller, _folder) = asked(json!({
        "type": "make_site", "name": "Blog", "host": "*.example.com", "first": "h1",
    }))
    .await;
    for _ in 0..5 {
        controller.make_requested_sites();
    }
    assert!(controller.presences().is_empty());
    finish(controller).await;
}
