//! Discord IPC frames: a little-endian opcode, a little-endian length, then JSON.

use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt};

/// Largest payload accepted from Discord.
pub const MAX_FRAME: usize = 1024 * 1024;

/// Operation codes of the IPC protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    /// First message, carries the client id.
    Handshake = 0,
    /// Commands and events.
    Frame = 1,
    /// The peer is closing the connection.
    Close = 2,
    /// Liveness probe; answer with `Pong` and the same payload.
    Ping = 3,
    /// Answer to `Ping`.
    Pong = 4,
}

impl Opcode {
    fn from_code(code: i32) -> Option<Self> {
        Some(match code {
            0 => Self::Handshake,
            1 => Self::Frame,
            2 => Self::Close,
            3 => Self::Ping,
            4 => Self::Pong,
            _ => return None,
        })
    }
}

/// Why a frame could not be read.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    /// The transport failed or closed in the middle of a frame.
    #[error("transport error: {0}")]
    Io(#[from] std::io::Error),
    /// The announced length is negative or above [`MAX_FRAME`].
    #[error("frame length {0} is not valid")]
    Length(i64),
    /// The opcode is not part of the protocol.
    #[error("unknown opcode {0}")]
    Opcode(i32),
    /// The payload is not JSON.
    #[error("payload is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// One frame ready to be written: the 8-byte header, then the JSON body.
pub fn encode(opcode: Opcode, payload: &Value) -> Vec<u8> {
    let body = payload.to_string().into_bytes();
    let length = i32::try_from(body.len()).unwrap_or(i32::MAX);
    let mut bytes = Vec::with_capacity(8 + body.len());
    bytes.extend_from_slice(&(opcode as i32).to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&body);
    bytes
}

/// Reads exactly one frame.
///
/// Raises:
///     FrameError: The stream fails, the length is above `MAX_FRAME`, the
///         opcode is unknown or the body is not JSON.
pub async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<(Opcode, Value), FrameError> {
    let mut header = [0_u8; 8];
    reader.read_exact(&mut header).await?;
    let code = i32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let length = i32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    let size = usize::try_from(length)
        .ok()
        .filter(|size| *size <= MAX_FRAME)
        .ok_or(FrameError::Length(length.into()))?;
    let opcode = Opcode::from_code(code).ok_or(FrameError::Opcode(code))?;
    let mut body = vec![0_u8; size];
    reader.read_exact(&mut body).await?;
    Ok((opcode, serde_json::from_slice(&body)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn frames_round_trip() {
        let payload = json!({"cmd": "SET_ACTIVITY", "text": "héllo"});
        let bytes = encode(Opcode::Frame, &payload);
        let (opcode, decoded) = read_frame(&mut bytes.as_slice()).await.unwrap();
        assert_eq!(opcode, Opcode::Frame);
        assert_eq!(decoded, payload);
    }

    #[tokio::test]
    async fn oversized_or_negative_lengths_are_refused() {
        for length in [MAX_FRAME as i32 + 1, -1] {
            let mut bytes = 1_i32.to_le_bytes().to_vec();
            bytes.extend_from_slice(&length.to_le_bytes());
            let result = read_frame(&mut bytes.as_slice()).await;
            assert!(matches!(result, Err(FrameError::Length(_))), "{length}");
        }
    }

    #[tokio::test]
    async fn unknown_opcode_and_bad_json_are_refused() {
        let mut bytes = 9_i32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&2_i32.to_le_bytes());
        bytes.extend_from_slice(b"{}");
        assert!(matches!(
            read_frame(&mut bytes.as_slice()).await,
            Err(FrameError::Opcode(9))
        ));
        let mut bytes = 1_i32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&3_i32.to_le_bytes());
        bytes.extend_from_slice(b"{no");
        assert!(matches!(
            read_frame(&mut bytes.as_slice()).await,
            Err(FrameError::Json(_))
        ));
    }

    #[tokio::test]
    async fn truncated_frames_are_an_io_error() {
        let bytes = encode(Opcode::Frame, &json!({"a": 1}));
        let cut = &bytes[..bytes.len() - 2];
        assert!(matches!(
            read_frame(&mut &cut[..]).await,
            Err(FrameError::Io(_))
        ));
    }
}
