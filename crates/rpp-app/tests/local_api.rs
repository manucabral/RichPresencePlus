//! The local address over a real socket.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;

use rpp_app::{Controller, Options};
use rpp_testkit::{Discord, Processes};

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

fn ask(port: u16, request: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    text
}

async fn finish(controller: Controller<Processes>) {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(controller.shutdown());
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn the_address_is_off_until_turned_on_and_closes_when_turned_off() {
    let (data, presences) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let controller = Controller::start(
        options(data.path(), presences.path()),
        Discord::default(),
        Processes(vec![]),
    )
    .await
    .unwrap();
    assert!(!controller.local_api().on);

    controller.set_local_api(true).unwrap();
    let view = controller.local_api();
    assert!(view.on && view.port != 0);
    let port = view.port;
    let request = format!("GET /v1/now HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
    let text = tokio::task::spawn_blocking(move || ask(port, &request))
        .await
        .unwrap();
    assert!(text.starts_with("HTTP/1.1 200"), "{text}");
    assert!(text.contains("\"showing\":false"), "{text}");

    controller.set_local_api(false).unwrap();
    assert!(!controller.local_api().on);
    let gone = tokio::task::spawn_blocking(move || {
        for _ in 0..50 {
            if TcpStream::connect(("127.0.0.1", port)).is_err() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    });
    assert!(gone.await.unwrap(), "the port stayed open");
    finish(controller).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_yes_comes_back_after_a_restart() {
    let (data, presences) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let controller = Controller::start(
        options(data.path(), presences.path()),
        Discord::default(),
        Processes(vec![]),
    )
    .await
    .unwrap();
    controller.set_local_api(true).unwrap();
    finish(controller).await;

    let again = Controller::start(
        options(data.path(), presences.path()),
        Discord::default(),
        Processes(vec![]),
    )
    .await
    .unwrap();
    let view = again.local_api();
    assert!(view.on);
    let port = view.port;
    let request = format!("GET /overlay HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n");
    let text = tokio::task::spawn_blocking(move || ask(port, &request))
        .await
        .unwrap();
    assert!(text.contains("<title>"), "{text}");
    finish(again).await;
}
