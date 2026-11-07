//! Discord IPC: framing, the activity command, one client and the supervised
//! connection that reconnects with a growing wait.

pub mod client;
pub mod connection;
pub mod frame;
pub mod protocol;

#[cfg(windows)]
pub mod pipe;

pub use client::{Client, Error};

/// Programs that serve Discord's IPC pipes: the official clients and the
/// alternative ones that speak the same protocol.
pub const CLIENT_EXECUTABLES: [&str; 6] = [
    "discord.exe",
    "discordcanary.exe",
    "discordptb.exe",
    "discorddevelopment.exe",
    "vesktop.exe",
    "legcord.exe",
];

/// True when a file name such as `Discord.exe` is a Discord client, ignoring case.
pub fn is_client_executable(name: &str) -> bool {
    CLIENT_EXECUTABLES
        .iter()
        .any(|known| known.eq_ignore_ascii_case(name))
}
pub use connection::{Connection, Connector};

/// Which Discord client receives the activity when several run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscordClient {
    /// The first one that answers, official or not.
    #[default]
    Auto,
    /// Discord Stable.
    Stable,
    /// Discord PTB.
    Ptb,
    /// Discord Canary.
    Canary,
}

impl DiscordClient {
    /// True when the program serving a pipe, such as `Discord.exe`, is the
    /// client this choice wants.
    pub fn serves(self, name: &str) -> bool {
        let wanted = match self {
            Self::Auto => return is_client_executable(name),
            Self::Stable => "discord.exe",
            Self::Ptb => "discordptb.exe",
            Self::Canary => "discordcanary.exe",
        };
        name.eq_ignore_ascii_case(wanted)
    }

    /// A number for storing the choice in an atomic; [`DiscordClient::from_code`] reads it back.
    pub fn code(self) -> u8 {
        match self {
            Self::Auto => 0,
            Self::Stable => 1,
            Self::Ptb => 2,
            Self::Canary => 3,
        }
    }

    /// The choice for a number from [`DiscordClient::code`]; anything else is automatic.
    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Stable,
            2 => Self::Ptb,
            3 => Self::Canary,
            _ => Self::Auto,
        }
    }
}

#[cfg(test)]
mod client_choice_tests {
    use super::*;

    #[test]
    fn each_choice_accepts_only_its_client() {
        assert!(DiscordClient::Auto.serves("Vesktop.exe"));
        assert!(DiscordClient::Auto.serves("DiscordPTB.exe"));
        assert!(!DiscordClient::Auto.serves("notdiscord.exe"));
        assert!(DiscordClient::Stable.serves("Discord.exe"));
        assert!(!DiscordClient::Stable.serves("DiscordCanary.exe"));
        assert!(DiscordClient::Ptb.serves("DISCORDPTB.EXE"));
        assert!(!DiscordClient::Ptb.serves("Discord.exe"));
        assert!(DiscordClient::Canary.serves("DiscordCanary.exe"));
        assert!(!DiscordClient::Canary.serves("vesktop.exe"));
    }

    #[test]
    fn codes_round_trip_and_unknown_ones_mean_automatic() {
        for choice in [
            DiscordClient::Auto,
            DiscordClient::Stable,
            DiscordClient::Ptb,
            DiscordClient::Canary,
        ] {
            assert_eq!(DiscordClient::from_code(choice.code()), choice);
        }
        assert_eq!(DiscordClient::from_code(200), DiscordClient::Auto);
    }
}
