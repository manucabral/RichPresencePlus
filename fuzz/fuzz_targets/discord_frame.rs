//! What a program posing as Discord can send on the pipe.
#![no_main]

use std::sync::LazyLock;

use libfuzzer_sys::fuzz_target;
use rpp_discord::frame::read_frame;
use tokio::runtime::Runtime;

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
});

fuzz_target!(|bytes: &[u8]| {
    let mut reader = bytes;
    let _ = RUNTIME.block_on(read_frame(&mut reader));
});
