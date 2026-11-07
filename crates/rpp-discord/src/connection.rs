//! A connection that opens itself, notices when Discord goes away and waits
//! longer after each failed attempt.

use std::io;
use std::time::{Duration, Instant};

use rpp_core::{Activity, Backoff};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::client::{Client, Error};

/// Opens the transport to a running Discord.
pub trait Connector {
    /// Byte stream to Discord.
    type Io: AsyncRead + AsyncWrite + Unpin;

    /// Tries to reach Discord once and returns the open byte stream.
    fn connect(&self) -> impl Future<Output = io::Result<Self::Io>>;
}

/// Why nothing was published.
#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    /// A recent attempt failed and the next one is not due yet.
    #[error("waiting {0:?} before reconnecting to Discord")]
    Waiting(Duration),
    /// Discord is not reachable.
    #[error("Discord is not running: {0}")]
    Unreachable(io::Error),
    /// Another program serves the connection that belongs to Discord.
    #[error("{0} holds the connection to Discord")]
    Taken(String),
    /// Discord answered with an error.
    #[error(transparent)]
    Discord(#[from] Error),
}

/// Publishes through one Discord application at a time.
pub struct Connection<C: Connector> {
    connector: C,
    client: Option<(String, Client<C::Io>)>,
    backoff: Backoff,
    /// Kept after the connection closes: the account rarely changes.
    user: Option<String>,
}

impl<C: Connector> Connection<C> {
    /// Wraps a connector; nothing is opened until the first publication.
    pub fn new(connector: C) -> Self {
        Self {
            connector,
            client: None,
            backoff: Backoff::new(Duration::from_secs(1), Duration::from_secs(30)),
            user: None,
        }
    }

    /// The user name of the Discord account seen at the last handshake.
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// True from a successful publication until the connection fails.
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Shows an activity for an application, or clears it.
    ///
    /// Raises:
    ///     PublishError: A recent attempt failed and its wait has not passed,
    ///         Discord is not reachable, or Discord answers with an error.
    pub async fn publish(
        &mut self,
        client_id: &str,
        activity: Option<&Activity>,
        now: Instant,
    ) -> Result<(), PublishError> {
        if self.client.as_ref().is_some_and(|(id, _)| id != client_id) {
            self.disconnect();
        }
        if self.client.is_none() {
            if !self.backoff.ready(now) {
                return Err(PublishError::Waiting(self.backoff.remaining(now)));
            }
            match self.open(client_id).await {
                Ok(client) => {
                    self.backoff.success();
                    if let Some(user) = client.user() {
                        self.user = Some(user.to_owned());
                    }
                    self.client = Some((client_id.to_owned(), client));
                }
                // Discord is fine and answered: one bad application must not
                // make the next presence wait.
                Err(error @ PublishError::Discord(Error::InvalidClient)) => return Err(error),
                Err(error) => {
                    self.backoff.failure(now);
                    return Err(error);
                }
            }
        }
        let Some((_, client)) = self.client.as_mut() else {
            return Ok(());
        };
        match client.set_activity(activity).await {
            Ok(()) => Ok(()),
            Err(error) => {
                if error.is_fatal() {
                    self.disconnect();
                    self.backoff.failure(now);
                }
                Err(error.into())
            }
        }
    }

    /// Asks Discord to confirm what it shows, and drops a connection that died.
    pub async fn confirm(&mut self, now: Instant) -> Result<(), PublishError> {
        let Some((_, client)) = self.client.as_mut() else {
            return Ok(());
        };
        client.confirm().await.map_err(|error| {
            if error.is_fatal() {
                self.client = None;
                self.backoff.failure(now);
            }
            error.into()
        })
    }

    async fn open(&self, client_id: &str) -> Result<Client<C::Io>, PublishError> {
        let io = self.connector.connect().await.map_err(|error| {
            if error.kind() == io::ErrorKind::PermissionDenied {
                PublishError::Taken(error.to_string())
            } else {
                PublishError::Unreachable(error)
            }
        })?;
        Ok(Client::handshake(io, client_id).await?)
    }

    /// Drops the connection; the next publication opens a new one.
    pub fn disconnect(&mut self) {
        self.client = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{Opcode, encode, read_frame};
    use serde_json::json;
    use std::cell::Cell;
    use tokio::io::{AsyncWriteExt, DuplexStream, duplex};

    /// Hands out in-memory streams, each served by a tiny fake Discord, or
    /// fails while `up` is false.
    struct Fake {
        up: Cell<bool>,
        attempts: Cell<u32>,
    }

    impl Fake {
        fn new(up: bool) -> Self {
            Self {
                up: Cell::new(up),
                attempts: Cell::new(0),
            }
        }
    }

    impl Connector for &Fake {
        type Io = DuplexStream;

        async fn connect(&self) -> io::Result<DuplexStream> {
            self.attempts.set(self.attempts.get() + 1);
            if !self.up.get() {
                return Err(io::ErrorKind::NotFound.into());
            }
            let (ours, mut theirs) = duplex(4096);
            tokio::spawn(async move {
                let Ok((_, hello)) = read_frame(&mut theirs).await else {
                    return;
                };
                let _ = hello;
                let ready = encode(Opcode::Frame, &json!({"evt": "READY"}));
                if theirs.write_all(&ready).await.is_err() {
                    return;
                }
                while let Ok((_, command)) = read_frame(&mut theirs).await {
                    let reply = json!({"evt": null, "nonce": command["nonce"]});
                    if theirs
                        .write_all(&encode(Opcode::Frame, &reply))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            });
            Ok(ours)
        }
    }

    fn shown(text: &str) -> Activity {
        Activity {
            details: Some(text.into()),
            ..Activity::default()
        }
    }

    #[tokio::test]
    async fn failed_attempts_wait_before_trying_again() {
        let fake = Fake::new(false);
        let mut connection = Connection::new(&fake);
        let now = Instant::now();
        let first = connection.publish("1", Some(&shown("Hi")), now).await;
        assert!(matches!(first, Err(PublishError::Unreachable(_))));
        let second = connection.publish("1", Some(&shown("Hi")), now).await;
        assert!(matches!(second, Err(PublishError::Waiting(_))));
        assert_eq!(fake.attempts.get(), 1);

        fake.up.set(true);
        let later = now + Duration::from_secs(2);
        connection
            .publish("1", Some(&shown("Hi")), later)
            .await
            .unwrap();
        assert!(connection.is_connected());
    }

    #[tokio::test]
    async fn a_new_application_id_opens_a_new_connection() {
        let fake = Fake::new(true);
        let mut connection = Connection::new(&fake);
        let now = Instant::now();
        connection
            .publish("1", Some(&shown("A1")), now)
            .await
            .unwrap();
        connection
            .publish("1", Some(&shown("B2")), now)
            .await
            .unwrap();
        assert_eq!(fake.attempts.get(), 1);
        connection
            .publish("2", Some(&shown("C3")), now)
            .await
            .unwrap();
        assert_eq!(fake.attempts.get(), 2);
    }
}
