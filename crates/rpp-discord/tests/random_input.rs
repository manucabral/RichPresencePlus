//! What a program posing as Discord can send: any bytes on the pipe.

use proptest::prelude::*;
use rpp_discord::frame::{Opcode, encode, read_frame};
use serde_json::json;

fn read(bytes: &[u8]) -> Result<(Opcode, serde_json::Value), impl std::error::Error> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let mut reader = bytes;
    runtime.block_on(read_frame(&mut reader))
}

proptest! {
    #[test]
    fn any_bytes_give_a_frame_or_an_error(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = read(&bytes);
    }

    #[test]
    fn a_header_that_claims_any_length_is_safe(
        code in any::<i32>(),
        length in any::<i32>(),
        body in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let mut bytes = code.to_le_bytes().to_vec();
        bytes.extend(length.to_le_bytes());
        bytes.extend(body);
        let _ = read(&bytes);
    }

    #[test]
    fn what_is_written_reads_back(text in "\\PC{0,200}", number in any::<i64>()) {
        let payload = json!({"text": text, "number": number});
        let (opcode, read_back) = read(&encode(Opcode::Frame, &payload)).unwrap();
        prop_assert_eq!(opcode, Opcode::Frame);
        prop_assert_eq!(read_back, payload);
    }
}
