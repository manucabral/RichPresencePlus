//! Naming the program behind a named pipe, as the Discord connection does.
#![cfg(windows)]

use std::os::windows::io::AsRawHandle;

use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};

#[tokio::test]
async fn the_server_of_a_pipe_is_named() {
    let name = format!(r"\\.\pipe\rpp-test-{}", std::process::id());
    let _server = ServerOptions::new().create(&name).unwrap();
    let client = ClientOptions::new().open(&name).unwrap();
    let served_by = rpp_host::pipe_server_executable(client.as_raw_handle()).unwrap();
    let this = std::env::current_exe().unwrap();
    assert_eq!(served_by, this.file_name().unwrap().to_string_lossy());
}
