//! What a browser writes to the native messaging host on its standard input.
#![no_main]

use libfuzzer_sys::fuzz_target;
use rpp_bridge::native::{answer, read_message};

fuzz_target!(|bytes: &[u8]| {
    let mut reader = bytes;
    if let Ok(message) = read_message(&mut reader, 1024) {
        assert!(message.len() <= 1024);
        let _ = answer(&message, None);
    }
    let _ = answer(bytes, None);
});
