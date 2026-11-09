//! What a browser extension, or a page posing as one, can send to the bridge.
#![no_main]

use libfuzzer_sys::fuzz_target;
use rpp_bridge::protocol::{Envelope, FromExtension};

fuzz_target!(|text: &str| {
    let _ = serde_json::from_str::<Envelope>(text);
    let _ = serde_json::from_str::<FromExtension>(text);
});
