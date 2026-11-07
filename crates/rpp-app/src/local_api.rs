//! The local read-only address, for overlays and stream decks.

use std::io;
use std::net::{Ipv4Addr, TcpListener as StdListener};
use std::sync::Arc;
use std::time::Duration;

use rpp_core::local_api::{MAX_REQUEST, answer, now_json};
use rpp_plugins::Host;
use rpp_service::Status;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, watch};
use tokio::task::AbortHandle;

use crate::controller::{Controller, lock};
use crate::views::LocalApiView;

/// Port of the address. It stays the same so an overlay keeps working.
pub(crate) const DEFAULT_PORT: u16 = 28752;
/// Requests served at once; an overlay and a stream deck are two.
const MAX_OPEN: usize = 8;
/// Time a connection may take. Whoever holds one open longer loses it.
const LIMIT: Duration = Duration::from_secs(5);

/// The address while it is on.
#[derive(Debug)]
pub(crate) struct Running {
    /// Stops the listening task.
    task: AbortHandle,
    /// The port it took.
    port: u16,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Reads what Discord shows from the service's status.
fn current(status: &Status) -> String {
    let delivered = status
        .activity
        .as_ref()
        .filter(|_| status.delivery_error.is_none());
    let name = status.rows.iter().find(|row| row.selected);
    now_json(name.map(|row| row.label.as_str()), delivered)
}

/// Reads one request and answers it.
async fn serve_one(
    mut stream: TcpStream,
    status: &watch::Receiver<Status>,
    port: u16,
) -> io::Result<()> {
    let mut request = vec![0; MAX_REQUEST];
    let mut read = 0;
    while read < MAX_REQUEST && !request[..read].windows(4).any(|end| end == b"\r\n\r\n") {
        let more = stream.read(&mut request[read..]).await?;
        if more == 0 {
            return Ok(());
        }
        read += more;
    }
    let now = current(&status.borrow());
    stream
        .write_all(&answer(&request[..read], port, &now))
        .await?;
    stream.shutdown().await
}

/// Accepts connections until the task is stopped.
async fn listen(listener: TcpListener, status: watch::Receiver<Status>, port: u16) {
    let open = Arc::new(Semaphore::new(MAX_OPEN));
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            // Out of sockets for a moment; not a reason to spin.
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        // Over the limit the connection closes: nothing here is worth queueing for.
        let Ok(slot) = Arc::clone(&open).try_acquire_owned() else {
            continue;
        };
        let status = status.clone();
        tokio::spawn(async move {
            let _slot = slot;
            let _ = tokio::time::timeout(LIMIT, serve_one(stream, &status, port)).await;
        });
    }
}

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// Takes the port and starts answering.
    pub(crate) fn start_local_api(&self) -> io::Result<()> {
        let mut running = lock(&self.local_api);
        if running.is_some() {
            return Ok(());
        }
        let listener = StdListener::bind((Ipv4Addr::LOCALHOST, self.api_port))?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let _inside = self.runtime.enter();
        let listener = TcpListener::from_std(listener)?;
        let task = self
            .runtime
            .spawn(listen(listener, self.service.watch(), port));
        *running = Some(Running {
            task: task.abort_handle(),
            port,
        });
        tracing::info!(port, "the local address is on");
        Ok(())
    }

    /// Returns whether the local address is on, and where.
    pub fn local_api(&self) -> LocalApiView {
        let port = lock(&self.local_api)
            .as_ref()
            .map_or(self.api_port, |running| running.port);
        LocalApiView {
            on: lock(&self.settings).local_api,
            port,
        }
    }

    /// Turns the local address on or off.
    ///
    /// Raises:
    ///     String: The port is taken or the setting cannot be saved.
    pub fn set_local_api(&self, on: bool) -> Result<(), String> {
        if on {
            self.start_local_api().map_err(|e| e.to_string())?;
        } else {
            *lock(&self.local_api) = None;
        }
        let mut settings = lock(&self.settings);
        let mut changed = settings.clone();
        changed.local_api = on;
        self.store.save(&changed).map_err(|e| e.to_string())?;
        *settings = changed;
        Ok(())
    }
}
