//! Mutual authentication and encryption of the extension connection.
//!
//! `extension/secure.js` is the other half and must stay in step with it.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Length of each random value, in bytes.
pub const NONCE_BYTES: usize = 32;

type HmacSha256 = Hmac<Sha256>;

/// Why a handshake step or a sealed message was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecureError {
    /// A value is not valid base64 or has the wrong length.
    #[error("malformed value")]
    Malformed,
    /// A proof does not match: the other side does not know the token.
    #[error("the proof does not match")]
    Proof,
    /// A sealed message failed to decrypt, or arrived out of order.
    #[error("a sealed message was refused")]
    Sealed,
}

/// Fresh random bytes from the operating system, as base64; `None` if the
/// system has no randomness to give.
pub fn new_nonce() -> Option<String> {
    let mut bytes = [0_u8; NONCE_BYTES];
    getrandom::fill(&mut bytes).ok()?;
    Some(STANDARD.encode(bytes))
}

fn decoded_nonce(text: &str) -> Result<(), SecureError> {
    match STANDARD.decode(text) {
        Ok(bytes) if bytes.len() == NONCE_BYTES => Ok(()),
        _ => Err(SecureError::Malformed),
    }
}

/// HMAC-SHA256 of `label|server|client` keyed with the token.
fn mac(token: &str, label: &str, server: &str, client: &str) -> HmacSha256 {
    // Any key length is valid for HMAC, so this cannot fail.
    let mut mac = <HmacSha256 as hmac::KeyInit>::new_from_slice(token.as_bytes())
        .unwrap_or_else(|_| unreachable!("HMAC accepts keys of any length"));
    mac.update(format!("{label}|{server}|{client}").as_bytes());
    mac
}

/// The base64 proof one side sends to show it knows the token.
///
/// Args:
///     label: `rpp-hello` for the extension, `rpp-welcome` for the application.
///     server: The application's random value, as base64.
///     client: The extension's random value, as base64.
pub fn proof(token: &str, label: &str, server: &str, client: &str) -> String {
    STANDARD.encode(mac(token, label, server, client).finalize().into_bytes())
}

/// Checks a proof in constant time.
///
/// Raises:
///     SecureError: `Malformed` for bad encoding, `Proof` when it does not match.
pub fn check_proof(
    token: &str,
    label: &str,
    server: &str,
    client: &str,
    given: &str,
) -> Result<(), SecureError> {
    decoded_nonce(server)?;
    decoded_nonce(client)?;
    let given = STANDARD.decode(given).map_err(|_| SecureError::Malformed)?;
    mac(token, label, server, client)
        .verify_slice(&given)
        .map_err(|_| SecureError::Proof)
}

/// One direction of a sealed conversation.
struct Direction {
    cipher: Aes256Gcm,
    counter: u64,
}

impl Direction {
    fn new(token: &str, label: &str, server: &str, client: &str) -> Self {
        let key = mac(token, label, server, client).finalize().into_bytes();
        Self {
            cipher: Aes256Gcm::new(&key),
            counter: 0,
        }
    }

    /// The 96-bit nonce for message `counter`: four zero bytes, then the counter.
    fn nonce(counter: u64) -> [u8; 12] {
        let mut nonce = [0_u8; 12];
        nonce[4..].copy_from_slice(&counter.to_be_bytes());
        nonce
    }
}

/// The keys of an authenticated connection, from one side's point of view.
pub struct Session {
    outgoing: Direction,
    incoming: Direction,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").finish_non_exhaustive()
    }
}

impl Session {
    /// The application's side of the session; `server` and `client` are the
    /// base64 random values of each side.
    pub fn for_application(token: &str, server: &str, client: &str) -> Self {
        Self {
            outgoing: Direction::new(token, "rpp-app-to-extension", server, client),
            incoming: Direction::new(token, "rpp-extension-to-app", server, client),
        }
    }

    /// The extension's side of the session, for tests and native clients.
    pub fn for_extension(token: &str, server: &str, client: &str) -> Self {
        Self {
            outgoing: Direction::new(token, "rpp-extension-to-app", server, client),
            incoming: Direction::new(token, "rpp-app-to-extension", server, client),
        }
    }

    /// Seals the JSON text of a message into the JSON text of an envelope.
    ///
    /// Raises:
    ///     SecureError: `Sealed` if the cipher refuses, or after 2^64 messages.
    pub fn seal(&mut self, plaintext: &str) -> Result<String, SecureError> {
        let counter = self.outgoing.counter;
        let nonce = Direction::nonce(counter);
        let sealed = self
            .outgoing
            .cipher
            .encrypt(&Nonce::from(nonce), plaintext.as_bytes())
            .map_err(|_| SecureError::Sealed)?;
        self.outgoing.counter = counter.checked_add(1).ok_or(SecureError::Sealed)?;
        Ok(serde_json::json!({
            "type": "sealed",
            "n": counter,
            "data": STANDARD.encode(sealed),
        })
        .to_string())
    }

    /// Opens the `n` and `data` of an envelope into the JSON text of a message.
    ///
    /// Raises:
    ///     SecureError: `Sealed` if it is out of order, repeated, altered or not text.
    pub fn open(&mut self, counter: u64, data: &str) -> Result<String, SecureError> {
        if counter != self.incoming.counter {
            return Err(SecureError::Sealed);
        }
        let sealed = STANDARD.decode(data).map_err(|_| SecureError::Sealed)?;
        let plain = self
            .incoming
            .cipher
            .decrypt(&Nonce::from(Direction::nonce(counter)), sealed.as_slice())
            .map_err(|_| SecureError::Sealed)?;
        self.incoming.counter = counter.checked_add(1).ok_or(SecureError::Sealed)?;
        String::from_utf8(plain).map_err(|_| SecureError::Sealed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "7c13347b8b6a4e8e9863498d5c534a5651cf8ee4565a419e95007e64b42b2bbc";

    fn nonces() -> (String, String) {
        (STANDARD.encode([1_u8; 32]), STANDARD.encode([2_u8; 32]))
    }

    /// The same values are checked by `extension/test/secure.test.js`, so both
    /// halves compute identical proofs.
    #[test]
    fn proofs_match_the_shared_test_vector() {
        let (server, client) = nonces();
        assert_eq!(
            proof(TOKEN, "rpp-hello", &server, &client),
            "FZysmTHImAUhRay3jFgUrcBKDODjQ+kz9W/gyQTksHY="
        );
    }

    /// Also checked by `extension/test/secure.test.js`: the first message the
    /// application seals opens in the extension, byte for byte.
    #[test]
    fn sealing_matches_the_shared_test_vector() {
        let (server, client) = nonces();
        let mut app = Session::for_application(TOKEN, &server, &client);
        let sealed: serde_json::Value =
            serde_json::from_str(&app.seal(r#"{"type":"pong"}"#).unwrap()).unwrap();
        assert_eq!(sealed["n"], 0);
        assert_eq!(
            sealed["data"],
            "0Fc5O/u3PEiDXU9ZEfkxsZbVM33OUSi4dnXfCLzGpg=="
        );
    }

    #[test]
    fn a_proof_needs_the_token_and_the_same_values() {
        let (server, client) = nonces();
        let good = proof(TOKEN, "rpp-hello", &server, &client);
        assert!(check_proof(TOKEN, "rpp-hello", &server, &client, &good).is_ok());
        assert_eq!(
            check_proof("other", "rpp-hello", &server, &client, &good),
            Err(SecureError::Proof)
        );
        assert_eq!(
            check_proof(TOKEN, "rpp-welcome", &server, &client, &good),
            Err(SecureError::Proof)
        );
        assert_eq!(
            check_proof(TOKEN, "rpp-hello", &client, &server, &good),
            Err(SecureError::Proof)
        );
        assert_eq!(
            check_proof(TOKEN, "rpp-hello", "short", &client, &good),
            Err(SecureError::Malformed)
        );
        assert_eq!(
            check_proof(TOKEN, "rpp-hello", &server, &client, "***"),
            Err(SecureError::Malformed)
        );
    }

    fn pair() -> (Session, Session) {
        let (server, client) = nonces();
        (
            Session::for_application(TOKEN, &server, &client),
            Session::for_extension(TOKEN, &server, &client),
        )
    }

    fn parts(envelope: &str) -> (u64, String) {
        let value: serde_json::Value = serde_json::from_str(envelope).unwrap();
        (
            value["n"].as_u64().unwrap(),
            value["data"].as_str().unwrap().to_owned(),
        )
    }

    #[test]
    fn messages_travel_both_ways_and_the_text_is_hidden() {
        let (mut app, mut extension) = pair();
        let sealed = app.seal(r#"{"type":"pong"}"#).unwrap();
        assert!(!sealed.contains("pong"));
        let (n, data) = parts(&sealed);
        assert_eq!(extension.open(n, &data).unwrap(), r#"{"type":"pong"}"#);

        let (n, data) = parts(&extension.seal("hello app").unwrap());
        assert_eq!(app.open(n, &data).unwrap(), "hello app");
    }

    #[test]
    fn replayed_reordered_or_altered_messages_are_refused() {
        let (mut app, mut extension) = pair();
        let first = parts(&extension.seal("one").unwrap());
        let second = parts(&extension.seal("two").unwrap());
        // Out of order.
        assert_eq!(app.open(second.0, &second.1), Err(SecureError::Sealed));
        assert_eq!(app.open(first.0, &first.1).unwrap(), "one");
        // Replayed.
        assert_eq!(app.open(first.0, &first.1), Err(SecureError::Sealed));
        // Altered.
        let mut bytes = STANDARD.decode(&second.1).unwrap();
        bytes[0] ^= 1;
        assert_eq!(
            app.open(second.0, &STANDARD.encode(bytes)),
            Err(SecureError::Sealed)
        );
        // A message the app sent cannot be fed back to the app.
        let echo = parts(&app.seal("mine").unwrap());
        let (mut app_again, _) = pair();
        assert_eq!(app_again.open(echo.0, &echo.1), Err(SecureError::Sealed));
    }

    #[test]
    fn nonces_are_random_and_well_formed() {
        let a = new_nonce().unwrap();
        let b = new_nonce().unwrap();
        assert_ne!(a, b);
        assert_eq!(STANDARD.decode(&a).unwrap().len(), NONCE_BYTES);
    }
}
