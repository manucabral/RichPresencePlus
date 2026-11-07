//! One handshaken Discord connection that publishes and clears an activity.

use std::time::Duration;

use rpp_core::Activity;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::time::timeout;

use crate::frame::{FrameError, Opcode, encode, read_frame};
use crate::protocol;

/// Longest wait for Discord to answer one request.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// Why a request to Discord failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The frame could not be written or read.
    #[error(transparent)]
    Frame(#[from] FrameError),
    /// Writing to the transport failed.
    #[error("transport error: {0}")]
    Io(#[from] std::io::Error),
    /// Discord did not answer in time.
    #[error("Discord did not answer in time")]
    Timeout,
    /// Discord closed the connection.
    #[error("Discord closed the connection: {0}")]
    Closed(String),
    /// Discord answered the handshake with something other than READY.
    #[error("handshake refused: {0}")]
    Handshake(String),
    /// Discord does not know the application of this presence. Other
    /// applications are not affected, so this says nothing about the connection.
    #[error("Discord does not know this application ID")]
    InvalidClient,
    /// Discord refused the command; the connection is still usable.
    #[error("Discord rejected the request: {0}")]
    Rejected(String),
}

impl Error {
    /// True when the connection can no longer be used: every error but a
    /// rejected request ends it.
    pub fn is_fatal(&self) -> bool {
        !matches!(self, Self::Rejected(_))
    }
}

/// Code of Discord's answer to a handshake with an unknown application.
const INVALID_CLIENT: i64 = 4000;

/// A connection that finished its handshake.
#[derive(Debug)]
pub struct Client<T> {
    io: T,
    pid: u32,
    shown: Option<Activity>,
    user: Option<String>,
    /// Group identifier sent with a party size; new for every connection.
    party: String,
}

/// Longest user name kept from the handshake; Discord's own limit is 32.
const MAX_USER: usize = 64;

/// The user name in Discord's READY answer, when it is plain text.
fn user_of(ready: &serde_json::Value) -> Option<String> {
    let name = ready["data"]["user"]["username"].as_str()?.trim();
    let plain =
        !name.is_empty() && name.chars().count() <= MAX_USER && !name.chars().any(char::is_control);
    plain.then(|| name.to_owned())
}

impl<T: AsyncRead + AsyncWrite + Unpin> Client<T> {
    /// Runs the handshake for an application over an open transport.
    ///
    /// Raises:
    ///     Error: Discord does not answer in time, closes the connection or
    ///         answers with something other than READY.
    pub async fn handshake(mut io: T, client_id: &str) -> Result<Self, Error> {
        let hello = encode(Opcode::Handshake, &protocol::handshake(client_id));
        timeout(REQUEST_TIMEOUT, io.write_all(&hello))
            .await
            .map_err(|_| Error::Timeout)??;
        let (opcode, reply) = timeout(REQUEST_TIMEOUT, read_frame(&mut io))
            .await
            .map_err(|_| Error::Timeout)??;
        if opcode == Opcode::Frame && reply["evt"] == "READY" {
            return Ok(Self {
                io,
                pid: std::process::id(),
                shown: None,
                user: user_of(&reply),
                party: uuid::Uuid::new_v4().simple().to_string(),
            });
        }
        if reply["code"] == INVALID_CLIENT {
            return Err(Error::InvalidClient);
        }
        Err(Error::Handshake(reply.to_string()))
    }

    /// The user name of the Discord account that answered, if it said one.
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// Shows an activity, or clears it.
    ///
    /// Raises:
    ///     Error: The request fails or Discord rejects it. A rejected activity
    ///         is not remembered, so the next call sends it again.
    pub async fn set_activity(&mut self, activity: Option<&Activity>) -> Result<(), Error> {
        if self.shown.as_ref() == activity {
            return Ok(());
        }
        self.send_activity(activity).await?;
        self.shown = activity.cloned();
        Ok(())
    }

    /// Sends the current activity again, to learn that Discord still answers.
    ///
    /// Raises:
    ///     Error: Discord is gone or does not answer in time.
    pub async fn confirm(&mut self) -> Result<(), Error> {
        let shown = self.shown.clone();
        self.send_activity(shown.as_ref()).await
    }

    async fn send_activity(&mut self, activity: Option<&Activity>) -> Result<(), Error> {
        let nonce = uuid::Uuid::new_v4().to_string();
        let command = protocol::set_activity(self.pid, activity, &nonce, &self.party);
        timeout(REQUEST_TIMEOUT, self.exchange(command, &nonce))
            .await
            .map_err(|_| Error::Timeout)?
    }

    /// Send a command and read until the answer carrying its nonce arrives.
    async fn exchange(&mut self, command: Value, nonce: &str) -> Result<(), Error> {
        self.io.write_all(&encode(Opcode::Frame, &command)).await?;
        loop {
            let (opcode, payload) = read_frame(&mut self.io).await?;
            match opcode {
                Opcode::Close => return Err(Error::Closed(payload.to_string())),
                Opcode::Ping => self.io.write_all(&encode(Opcode::Pong, &payload)).await?,
                Opcode::Frame if payload["nonce"] == nonce => {
                    if payload["evt"] == "ERROR" {
                        return Err(Error::Rejected(payload["data"].to_string()));
                    }
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::{DuplexStream, duplex};

    #[test]
    fn the_user_name_is_read_from_the_ready_answer_when_it_is_plain() {
        let ready =
            |name: serde_json::Value| json!({"evt": "READY", "data": {"user": {"username": name}}});
        assert_eq!(
            user_of(&ready(json!("manucabral"))).as_deref(),
            Some("manucabral")
        );
        assert_eq!(user_of(&ready(json!("  ana  "))).as_deref(), Some("ana"));
        for bad in [json!(""), json!("a\nb"), json!(7), json!("x".repeat(65))] {
            assert_eq!(user_of(&ready(bad.clone())), None, "{bad}");
        }
        assert_eq!(user_of(&json!({"evt": "READY"})), None);
    }

    /// Stand-in for Discord: answers the handshake, then each command.
    async fn fake_discord(mut io: DuplexStream, replies: Vec<&'static str>) {
        let (_, hello) = read_frame(&mut io).await.unwrap();
        assert_eq!(hello["client_id"], "123");
        io.write_all(&encode(Opcode::Frame, &json!({"evt": "READY"})))
            .await
            .unwrap();
        for reply in replies {
            let Ok((_, command)) = read_frame(&mut io).await else {
                return;
            };
            let nonce = command["nonce"].clone();
            let body = match reply {
                "ok" => json!({"cmd": "SET_ACTIVITY", "evt": null, "nonce": nonce}),
                "error" => json!({"evt": "ERROR", "data": {"code": 4000}, "nonce": nonce}),
                _ => return,
            };
            io.write_all(&encode(Opcode::Frame, &body)).await.unwrap();
        }
    }

    fn activity(text: &str) -> Activity {
        Activity {
            details: Some(text.into()),
            ..Activity::default()
        }
    }

    #[tokio::test]
    async fn publishes_clears_and_skips_repeats() {
        let (ours, theirs) = duplex(4096);
        // Two replies only: the repeated activity must not reach Discord.
        let server = tokio::spawn(fake_discord(theirs, vec!["ok", "ok"]));
        let mut client = Client::handshake(ours, "123").await.unwrap();
        let shown = activity("Hello");
        client.set_activity(Some(&shown)).await.unwrap();
        client.set_activity(Some(&shown)).await.unwrap();
        client.set_activity(None).await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn rejection_is_not_fatal_and_is_not_cached() {
        let (ours, theirs) = duplex(4096);
        let server = tokio::spawn(fake_discord(theirs, vec!["error", "ok"]));
        let mut client = Client::handshake(ours, "123").await.unwrap();
        let shown = activity("Hello");
        let error = client.set_activity(Some(&shown)).await.unwrap_err();
        assert!(matches!(error, Error::Rejected(_)));
        assert!(!error.is_fatal());
        // The rejected activity was not remembered, so it is sent again.
        client.set_activity(Some(&shown)).await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn ping_is_answered_while_waiting() {
        let (ours, mut theirs) = duplex(4096);
        let server = tokio::spawn(async move {
            read_frame(&mut theirs).await.unwrap();
            let ready = encode(Opcode::Frame, &json!({"evt": "READY"}));
            theirs.write_all(&ready).await.unwrap();
            let (_, command) = read_frame(&mut theirs).await.unwrap();
            theirs
                .write_all(&encode(Opcode::Ping, &json!({"n": 1})))
                .await
                .unwrap();
            let (opcode, echoed) = read_frame(&mut theirs).await.unwrap();
            assert_eq!((opcode, echoed), (Opcode::Pong, json!({"n": 1})));
            let done = json!({"evt": null, "nonce": command["nonce"]});
            theirs
                .write_all(&encode(Opcode::Frame, &done))
                .await
                .unwrap();
        });
        let mut client = Client::handshake(ours, "123").await.unwrap();
        client.set_activity(Some(&activity("Hi"))).await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn close_and_bad_handshake_are_fatal() {
        let (ours, mut theirs) = duplex(4096);
        let server = tokio::spawn(async move {
            read_frame(&mut theirs).await.unwrap();
            let refusal = json!({"code": 4009, "message": "Invalid Client ID"});
            theirs
                .write_all(&encode(Opcode::Close, &refusal))
                .await
                .unwrap();
        });
        let error = Client::handshake(ours, "123").await.unwrap_err();
        assert!(matches!(error, Error::Handshake(_)));
        assert!(error.is_fatal());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn a_closed_discord_fails_the_confirmation() {
        let (ours, mut theirs) = duplex(4096);
        let server = tokio::spawn(async move {
            read_frame(&mut theirs).await.unwrap();
            let ready = json!({"evt": "READY"});
            theirs
                .write_all(&encode(Opcode::Frame, &ready))
                .await
                .unwrap();
        });
        let mut client = Client::handshake(ours, "123").await.unwrap();
        server.await.unwrap();
        let error = client.confirm().await.unwrap_err();
        assert!(error.is_fatal());
    }

    #[tokio::test]
    async fn an_unknown_application_is_told_apart_from_other_refusals() {
        let (ours, mut theirs) = duplex(4096);
        let server = tokio::spawn(async move {
            read_frame(&mut theirs).await.unwrap();
            let refusal = json!({"code": 4000, "message": "Invalid Client ID"});
            theirs
                .write_all(&encode(Opcode::Close, &refusal))
                .await
                .unwrap();
        });
        let error = Client::handshake(ours, "123").await.unwrap_err();
        assert!(matches!(error, Error::InvalidClient));
        server.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn silence_times_out() {
        let (ours, mut theirs) = duplex(4096);
        tokio::spawn(async move {
            read_frame(&mut theirs).await.unwrap();
            let ready = encode(Opcode::Frame, &json!({"evt": "READY"}));
            theirs.write_all(&ready).await.unwrap();
            // Never answers the command.
            std::future::pending::<()>().await;
        });
        let mut client = Client::handshake(ours, "123").await.unwrap();
        let error = client
            .set_activity(Some(&activity("Hi")))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Timeout));
    }
}
