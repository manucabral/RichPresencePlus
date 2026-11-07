//! Starting the application with Windows.

use std::path::Path;

use windows_registry::CURRENT_USER;

/// Where Windows looks for programs to start at sign-in.
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// Name of the value this application owns under [`RUN_KEY`].
const VALUE_NAME: &str = "RichPresencePlus";
/// Key the installer writes once it has asked about starting with Windows.
const MARKER_KEY: &str = r"Software\manucabral\Rich Presence Plus";
const MARKER_VALUE: &str = "StartupAsked";
/// `ERROR_FILE_NOT_FOUND` as an HRESULT: the value does not exist.
const NOT_FOUND: i32 = 0x8007_0002_u32 as i32;
/// Argument that keeps the window hidden at start.
pub const HIDDEN_ARGUMENT: &str = "--hidden";

/// Whether this program starts with Windows, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StartupState {
    /// Windows starts this very program at sign-in.
    pub enabled: bool,
    /// It starts hidden in the tray.
    pub hidden: bool,
}

/// The program and the arguments of a Run entry.
fn parse(data: &str) -> (&str, &str) {
    let data = data.trim();
    match data.strip_prefix('"').and_then(|rest| rest.split_once('"')) {
        Some((program, arguments)) => (program, arguments.trim()),
        None => data.split_once(' ').unwrap_or((data, "")),
    }
}

fn same_program(listed: &str, exe: &Path) -> bool {
    listed.to_lowercase() == exe.to_string_lossy().to_lowercase()
}

/// The Run entry of this application.
#[derive(Debug, Clone)]
pub struct Startup {
    key: String,
    value: String,
}

impl Startup {
    /// The entry Windows reads at sign-in.
    pub fn system() -> Self {
        Self {
            key: RUN_KEY.into(),
            value: VALUE_NAME.into(),
        }
    }

    /// Reads the entry and checks it starts `exe`.
    pub fn state(&self, exe: &Path) -> StartupState {
        let Ok(data) = CURRENT_USER
            .open(&self.key)
            .and_then(|key| key.get_string(&self.value))
        else {
            return StartupState::default();
        };
        let (program, arguments) = parse(&data);
        if !same_program(program, exe) {
            return StartupState::default();
        }
        StartupState {
            enabled: true,
            hidden: arguments
                .split_whitespace()
                .any(|argument| argument == HIDDEN_ARGUMENT),
        }
    }

    /// Makes `exe` start with Windows, or stops it.
    ///
    /// Args:
    ///     exe: The running program, never a path from the window.
    ///     start: `None` removes the entry; `Some(hidden)` writes it.
    pub fn set(&self, exe: &Path, start: Option<bool>) -> Result<(), String> {
        let key = CURRENT_USER.create(&self.key).map_err(|e| e.to_string())?;
        let Some(hidden) = start else {
            return match key.remove_value(&self.value) {
                Ok(()) => Ok(()),
                // Already gone is what was asked for.
                Err(error) if error.code().0 == NOT_FOUND => Ok(()),
                Err(error) => Err(error.to_string()),
            };
        };
        // A replaced character would leave an entry that starts nothing.
        let program = exe
            .to_str()
            .filter(|path| !path.contains('"'))
            .ok_or("The program path cannot be written to the startup entry.")?;
        let data = if hidden {
            format!("\"{program}\" {HIDDEN_ARGUMENT}")
        } else {
            format!("\"{program}\"")
        };
        key.set_string(&self.value, data).map_err(|e| e.to_string())
    }
}

/// Tells whether the installer already asked about starting with Windows.
pub fn asked_by_installer() -> bool {
    CURRENT_USER
        .open(MARKER_KEY)
        .and_then(|key| key.get_u32(MARKER_VALUE))
        .is_ok_and(|value| value == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A key of its own directly under `Software`, removed whole when the test ends.
    struct Scratch(Startup);

    impl Scratch {
        fn new() -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(Startup {
                key: format!(
                    r"Software\RichPresencePlusTest-{}-{nanos}",
                    std::process::id()
                ),
                value: VALUE_NAME.into(),
            })
        }

        fn raw(&self) -> Option<String> {
            CURRENT_USER
                .open(&self.0.key)
                .and_then(|key| key.get_string(VALUE_NAME))
                .ok()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = CURRENT_USER.remove_tree(&self.0.key);
        }
    }

    fn exe() -> PathBuf {
        PathBuf::from(r"C:\Program Files\Rich Presence Plus\rpp-desktop.exe")
    }

    #[test]
    fn the_entry_is_written_read_and_removed() {
        let scratch = Scratch::new();
        assert_eq!(scratch.0.state(&exe()), StartupState::default());

        scratch.0.set(&exe(), Some(true)).unwrap();
        assert_eq!(
            scratch.raw().as_deref(),
            Some(r#""C:\Program Files\Rich Presence Plus\rpp-desktop.exe" --hidden"#)
        );
        assert_eq!(
            scratch.0.state(&exe()),
            StartupState {
                enabled: true,
                hidden: true
            }
        );

        scratch.0.set(&exe(), Some(false)).unwrap();
        assert_eq!(
            scratch.0.state(&exe()),
            StartupState {
                enabled: true,
                hidden: false
            }
        );

        scratch.0.set(&exe(), None).unwrap();
        assert_eq!(scratch.raw(), None);
        // Removing what is not there is not an error.
        scratch.0.set(&exe(), None).unwrap();
    }

    #[test]
    fn an_entry_for_another_copy_of_the_program_counts_as_off() {
        let scratch = Scratch::new();
        scratch
            .0
            .set(Path::new(r"D:\old\rpp-desktop.exe"), Some(true))
            .unwrap();
        assert_eq!(scratch.0.state(&exe()), StartupState::default());
        // The same path in other letter case is the same program.
        let upper = PathBuf::from(exe().to_string_lossy().to_uppercase());
        scratch.0.set(&upper, Some(true)).unwrap();
        assert!(scratch.0.state(&exe()).enabled);
    }

    #[test]
    fn a_path_with_quotes_is_refused() {
        let scratch = Scratch::new();
        assert!(
            scratch
                .0
                .set(Path::new(r#"C:\a" --evil "\b.exe"#), Some(false))
                .is_err()
        );
        assert_eq!(scratch.raw(), None);
    }

    #[test]
    fn entries_are_split_into_program_and_arguments() {
        assert_eq!(
            parse(r#""C:\a b\c.exe" --hidden"#),
            (r"C:\a b\c.exe", "--hidden")
        );
        assert_eq!(parse(r#""C:\a b\c.exe""#), (r"C:\a b\c.exe", ""));
        assert_eq!(parse(r"C:\a\c.exe --x"), (r"C:\a\c.exe", "--x"));
        assert_eq!(parse("  "), ("", ""));
        assert_eq!(parse(r#""unterminated"#), (r#""unterminated"#, ""));
    }
}
