//! Discord's named pipes on Windows.

use std::io;

use tokio::net::windows::named_pipe::ClientOptions;
pub use tokio::net::windows::named_pipe::NamedPipeClient;

use crate::connection::Connector;

/// How many `discord-ipc-N` pipes Discord may have opened.
const PIPE_COUNT: u8 = 10;

/// Name of Discord's pipes, followed by a number.
const PIPE_NAME: &str = r"\\.\pipe\discord-ipc-";

/// Decides whether the program behind an opened pipe may receive activities;
/// the error names the program when it may not.
pub type ServerCheck = fn(&NamedPipeClient) -> Result<(), String>;

/// Opens the first Discord pipe that answers.
#[derive(Debug, Default, Clone, Copy)]
pub struct PipeConnector {
    check: Option<ServerCheck>,
}

impl PipeConnector {
    /// A connector that trusts any program serving the pipe.
    pub fn new() -> Self {
        Self::default()
    }

    /// A connector that skips pipes whose server fails `check`.
    pub fn verified(check: ServerCheck) -> Self {
        Self { check: Some(check) }
    }
}

impl Connector for PipeConnector {
    type Io = NamedPipeClient;

    /// Opens the first Discord pipe that accepts a connection.
    ///
    /// Raises:
    ///     io::Error: No `discord-ipc-N` pipe can be opened. Its kind is
    ///         `PermissionDenied`, with the name of the program as its text, if
    ///         a pipe answered but belongs to another program.
    async fn connect(&self) -> io::Result<NamedPipeClient> {
        self.connect_to(PIPE_NAME)
    }
}

impl PipeConnector {
    fn connect_to(&self, name: &str) -> io::Result<NamedPipeClient> {
        let mut last = io::Error::from(io::ErrorKind::NotFound);
        let mut taken = None;
        for index in 0..PIPE_COUNT {
            match ClientOptions::new().open(format!("{name}{index}")) {
                Ok(pipe) => match self.check.map_or(Ok(()), |check| check(&pipe)) {
                    Ok(()) => return Ok(pipe),
                    Err(program) => taken = taken.or(Some(program)),
                },
                // Not there, or busy with another client: try the next one.
                Err(error) => last = error,
            }
        }
        // The empty pipes after a taken one say nothing: what the user needs to
        // hear is which program stands in the way.
        Err(taken.map_or(last, |program| {
            io::Error::new(io::ErrorKind::PermissionDenied, program)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::windows::named_pipe::ServerOptions;

    fn other_program(_: &NamedPipeClient) -> Result<(), String> {
        Err("Fake.exe".to_owned())
    }

    #[tokio::test]
    async fn a_taken_pipe_is_named_even_with_empty_pipes_after_it() {
        let name = format!(r"\\.\pipe\rpp-test-{}-", std::process::id());
        let _server = ServerOptions::new().create(format!("{name}0")).unwrap();
        let error = PipeConnector::verified(other_program)
            .connect_to(&name)
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "Fake.exe");
    }

    #[tokio::test]
    async fn a_good_pipe_after_a_taken_one_is_used() {
        fn only_the_second(pipe: &NamedPipeClient) -> Result<(), String> {
            use std::sync::atomic::{AtomicBool, Ordering};
            static SEEN: AtomicBool = AtomicBool::new(false);
            let _ = pipe;
            if SEEN.swap(true, Ordering::SeqCst) {
                Ok(())
            } else {
                Err("Fake.exe".to_owned())
            }
        }
        let name = format!(r"\\.\pipe\rpp-test-good-{}-", std::process::id());
        let _first = ServerOptions::new().create(format!("{name}0")).unwrap();
        let _second = ServerOptions::new().create(format!("{name}1")).unwrap();
        assert!(
            PipeConnector::verified(only_the_second)
                .connect_to(&name)
                .is_ok()
        );
    }

    #[tokio::test]
    async fn no_pipe_at_all_says_so() {
        let name = format!(r"\\.\pipe\rpp-test-none-{}-", std::process::id());
        let error = PipeConnector::new().connect_to(&name).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
