//! A browser extension for tests: pairs with the bridge and talks to it sealed.

use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use rpp_bridge::VERSION;
use rpp_bridge::secure::{self, Session};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A paired extension.
pub struct Extension {
    socket: Socket,
    session: Session,
}

async fn next_text(socket: &mut Socket) -> Value {
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => return serde_json::from_str(text.as_str()).unwrap(),
            Some(Ok(_)) => {}
            other => panic!("the bridge closed the connection: {other:?}"),
        }
    }
}

impl Extension {
    /// Connects as a Chrome extension, completes the handshake and waits for
    /// the first rules.
    pub async fn pair(address: SocketAddr, token: &str) -> Self {
        let mut request = format!("ws://{address}/").into_client_request().unwrap();
        request.headers_mut().insert(
            "origin",
            "chrome-extension://abcdefghijklmnop".parse().unwrap(),
        );
        let (mut socket, _) = connect_async(request).await.unwrap();
        let challenge = next_text(&mut socket).await;
        let server = challenge["nonce"].as_str().unwrap().to_owned();
        let client = secure::new_nonce().unwrap();
        let hello = json!({
            "type": "hello", "browser": "chrome", "version": VERSION, "nonce": client,
            "proof": secure::proof(token, "rpp-hello", &server, &client),
        });
        socket.send(Message::text(hello.to_string())).await.unwrap();
        assert_eq!(next_text(&mut socket).await["type"], "welcome");
        let mut extension = Self {
            socket,
            session: Session::for_extension(token, &server, &client),
        };
        assert_eq!(extension.receive().await["type"], "rules");
        extension
    }

    /// Sends a message sealed.
    pub async fn send(&mut self, message: Value) {
        let sealed = self.session.seal(&message.to_string()).unwrap();
        self.socket.send(Message::text(sealed)).await.unwrap();
    }

    /// Waits for the next message and opens it.
    pub async fn receive(&mut self) -> Value {
        let envelope = next_text(&mut self.socket).await;
        let n = envelope["n"].as_u64().unwrap();
        let text = self
            .session
            .open(n, envelope["data"].as_str().unwrap())
            .unwrap();
        serde_json::from_str(&text).unwrap()
    }
}
