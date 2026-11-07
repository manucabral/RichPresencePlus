//! What any local program can send to the bridge and the native host.

use proptest::prelude::*;
use rpp_bridge::native::{answer, read_message};
use rpp_bridge::secure::{self, Session};

const TOKEN: &str = "7c13347b8b6a4e8e9863498d5c534a5651cf8ee4565a419e95007e64b42b2bbc";
const SERVER: &str = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=";
const CLIENT: &str = "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=";

fn app() -> Session {
    Session::for_application(TOKEN, SERVER, CLIENT)
}

fn extension() -> Session {
    Session::for_extension(TOKEN, SERVER, CLIENT)
}

/// The counter and data of a sealed envelope.
fn parts(envelope: &str) -> (u64, String) {
    let value: serde_json::Value = serde_json::from_str(envelope).unwrap();
    (
        value["n"].as_u64().unwrap(),
        value["data"].as_str().unwrap().to_owned(),
    )
}

proptest! {
    #[test]
    fn made_up_messages_never_open(counter in any::<u64>(), data in "\\PC{0,300}") {
        prop_assert!(app().open(counter, &data).is_err());
    }

    #[test]
    fn what_is_sealed_opens_once(text in "\\PC{0,400}") {
        let (counter, data) = parts(&extension().seal(&text).unwrap());
        let mut receiver = app();
        prop_assert_eq!(receiver.open(counter, &data).unwrap(), text);
        prop_assert!(receiver.open(counter, &data).is_err());
    }

    #[test]
    fn a_changed_byte_is_refused(text in "\\PC{1,100}", at in any::<prop::sample::Index>(), flip in 1_u8..=255) {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;
        let (counter, data) = parts(&extension().seal(&text).unwrap());
        let mut bytes = STANDARD.decode(data).unwrap();
        let position = at.index(bytes.len());
        bytes[position] ^= flip;
        prop_assert!(app().open(counter, &STANDARD.encode(bytes)).is_err());
    }

    #[test]
    fn a_seal_from_another_token_is_refused(text in "\\PC{0,100}", other in "[0-9a-f]{64}") {
        prop_assume!(other != TOKEN);
        let (counter, data) = parts(&Session::for_extension(&other, SERVER, CLIENT).seal(&text).unwrap());
        prop_assert!(app().open(counter, &data).is_err());
    }

    #[test]
    fn made_up_proofs_are_refused(proof in "\\PC{0,120}") {
        prop_assert!(secure::check_proof(TOKEN, "rpp-hello", SERVER, CLIENT, &proof).is_err());
    }

    #[test]
    fn the_native_host_survives_any_input(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let mut reader = bytes.as_slice();
        if let Ok(message) = read_message(&mut reader, 1024) {
            prop_assert!(message.len() <= 1024);
        }
        let _ = answer(&bytes, None);
    }

    #[test]
    fn a_declared_length_past_the_limit_reads_nothing(length in 1025_u32.., body in proptest::collection::vec(any::<u8>(), 0..64)) {
        let mut bytes = length.to_le_bytes().to_vec();
        bytes.extend(body);
        prop_assert!(read_message(&mut bytes.as_slice(), 1024).is_err());
    }
}
