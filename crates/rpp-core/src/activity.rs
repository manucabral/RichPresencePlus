//! The activity shown on Discord and the rules every activity must satisfy.

use serde::{Deserialize, Serialize};
use url::Url;

/// Longest text accepted for `state`, `details` and the image captions.
pub const MAX_TEXT: usize = 128;
/// Discord refuses an activity that carries a text shorter than this.
pub const MIN_TEXT: usize = 2;
/// Longest image key or external image address.
pub const MAX_IMAGE: usize = 512;
/// Longest button label.
pub const MAX_LABEL: usize = 32;
/// Longest button address.
pub const MAX_BUTTON_URL: usize = 512;
/// Discord shows at most this many buttons.
pub const MAX_BUTTONS: usize = 2;
/// Largest group size accepted in `party_size`.
pub const MAX_PARTY: u32 = 1000;

/// How Discord words the activity ("Playing", "Listening to", ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityKind {
    /// "Playing ..."
    #[default]
    Playing,
    /// "Listening to ..."
    Listening,
    /// "Watching ..."
    Watching,
    /// "Competing in ..."
    Competing,
}

impl ActivityKind {
    /// Discord's numeric code: 0 playing, 2 listening, 3 watching, 5 competing.
    pub fn code(self) -> u8 {
        match self {
            Self::Playing => 0,
            Self::Listening => 2,
            Self::Watching => 3,
            Self::Competing => 5,
        }
    }
}

/// Which text the member list shows next to the user's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusDisplay {
    /// The application name: "Playing Rich Presence Plus".
    Name,
    /// The `state` line.
    State,
    /// The `details` line.
    Details,
}

impl StatusDisplay {
    /// Discord's `status_display_type`: 0 name, 1 state, 2 details.
    pub fn code(self) -> u8 {
        match self {
            Self::Name => 0,
            Self::State => 1,
            Self::Details => 2,
        }
    }
}

/// A link button under the activity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Button {
    /// Text on the button.
    pub label: String,
    /// Address opened by the button; HTTP or HTTPS only.
    pub url: String,
}

/// One rich presence payload, before it is filtered by the user's privacy rules.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Activity {
    /// Title of the activity, in place of the Discord application's name. It lets
    /// presences share one application and still show as themselves.
    pub name: Option<String>,
    /// Second line.
    pub state: Option<String>,
    /// First line.
    pub details: Option<String>,
    /// How Discord words the activity.
    pub kind: ActivityKind,
    /// Start of the elapsed counter, in Unix seconds.
    pub start: Option<u64>,
    /// End of the remaining counter, in Unix seconds.
    pub end: Option<u64>,
    /// Large image key or external address.
    pub large_image: Option<String>,
    /// Caption of the large image.
    pub large_text: Option<String>,
    /// Small image key or external address.
    pub small_image: Option<String>,
    /// Caption of the small image.
    pub small_text: Option<String>,
    /// Up to two link buttons.
    pub buttons: Vec<Button>,
    /// Current and largest size of the group, shown as "(2 of 4)".
    pub party_size: Option<[u32; 2]>,
    /// Which text the member list shows; Discord shows the name when unset.
    pub status_display: Option<StatusDisplay>,
    /// Opened when clicking `details`.
    pub details_url: Option<String>,
    /// Opened when clicking `state`.
    pub state_url: Option<String>,
    /// Opened when clicking the large image.
    pub large_url: Option<String>,
    /// Opened when clicking the small image.
    pub small_url: Option<String>,
}

/// Why an activity was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActivityError {
    /// A text field is longer than Discord allows.
    #[error("{field} must have at most {limit} characters")]
    TooLong {
        /// Name of the offending field.
        field: &'static str,
        /// Longest accepted length.
        limit: usize,
    },
    /// More buttons than Discord displays.
    #[error("use at most {MAX_BUTTONS} buttons")]
    TooManyButtons,
    /// A button label is empty or too long.
    #[error("button label must have 1 to {MAX_LABEL} characters")]
    ButtonLabel,
    /// A button address is not a plain HTTP or HTTPS link.
    #[error("button address must be HTTP or HTTPS without credentials")]
    ButtonUrl,
    /// The end of the counter comes before its start.
    #[error("end must not be earlier than start")]
    Timestamps,
    /// A link on a text or an image is not a plain HTTP or HTTPS address.
    #[error("{field} must be an HTTP or HTTPS address without credentials")]
    Link {
        /// Name of the offending field.
        field: &'static str,
    },
    /// The group size is outside 1 ≤ current ≤ largest ≤ `MAX_PARTY`.
    #[error("party_size must be [current, largest] with 1 <= current <= largest <= {MAX_PARTY}")]
    Party,
}

fn clean(
    field: &'static str,
    value: &mut Option<String>,
    limit: usize,
) -> Result<(), ActivityError> {
    let Some(text) = value else {
        return Ok(());
    };
    let length = text.chars().count();
    if length > limit {
        return Err(ActivityError::TooLong { field, limit });
    }
    if length < MIN_TEXT {
        // A single character would make Discord refuse the whole activity.
        *value = None;
    }
    Ok(())
}

fn check_image(field: &'static str, value: &mut Option<String>) -> Result<(), ActivityError> {
    match value {
        Some(text) if text.chars().count() > MAX_IMAGE => Err(ActivityError::TooLong {
            field,
            limit: MAX_IMAGE,
        }),
        Some(text) if text.is_empty() => {
            *value = None;
            Ok(())
        }
        _ => Ok(()),
    }
}

/// An HTTP or HTTPS address with a host and no user name or password.
fn plain_link(address: &str) -> bool {
    address.chars().count() <= MAX_BUTTON_URL
        && Url::parse(address).is_ok_and(|parsed| {
            matches!(parsed.scheme(), "http" | "https")
                && parsed.host_str().is_some()
                && parsed.username().is_empty()
                && parsed.password().is_none()
        })
}

fn check_button(button: &Button) -> Result<(), ActivityError> {
    let label = button.label.chars().count();
    if !(1..=MAX_LABEL).contains(&label) {
        return Err(ActivityError::ButtonLabel);
    }
    if plain_link(&button.url) {
        Ok(())
    } else {
        Err(ActivityError::ButtonUrl)
    }
}

fn check_link(field: &'static str, value: &mut Option<String>) -> Result<(), ActivityError> {
    match value {
        Some(text) if text.is_empty() => {
            *value = None;
            Ok(())
        }
        Some(text) if !plain_link(text) => Err(ActivityError::Link { field }),
        _ => Ok(()),
    }
}

impl Activity {
    /// Checks every field and returns the activity Discord will accept. Texts
    /// shorter than `MIN_TEXT` are dropped, not rejected.
    ///
    /// Raises:
    ///     ActivityError: The first field that breaks a Discord limit.
    pub fn validated(mut self) -> Result<Self, ActivityError> {
        clean("name", &mut self.name, MAX_TEXT)?;
        clean("state", &mut self.state, MAX_TEXT)?;
        clean("details", &mut self.details, MAX_TEXT)?;
        clean("large_text", &mut self.large_text, MAX_TEXT)?;
        clean("small_text", &mut self.small_text, MAX_TEXT)?;
        check_image("large_image", &mut self.large_image)?;
        check_image("small_image", &mut self.small_image)?;
        if self.buttons.len() > MAX_BUTTONS {
            return Err(ActivityError::TooManyButtons);
        }
        self.buttons.iter().try_for_each(check_button)?;
        check_link("details_url", &mut self.details_url)?;
        check_link("state_url", &mut self.state_url)?;
        check_link("large_url", &mut self.large_url)?;
        check_link("small_url", &mut self.small_url)?;
        if let Some([current, largest]) = self.party_size
            && !(current >= 1 && current <= largest && largest <= MAX_PARTY)
        {
            return Err(ActivityError::Party);
        }
        if let (Some(start), Some(end)) = (self.start, self.end)
            && end < start
        {
            return Err(ActivityError::Timestamps);
        }
        self.drop_orphans();
        Ok(self)
    }

    /// Every address the activity links to: buttons and clickable texts and images.
    pub fn links(&self) -> impl Iterator<Item = &str> {
        self.buttons.iter().map(|button| button.url.as_str()).chain(
            [
                &self.details_url,
                &self.state_url,
                &self.large_url,
                &self.small_url,
            ]
            .into_iter()
            .filter_map(|link| link.as_deref()),
        )
    }

    /// Removes the links on texts and images.
    pub fn clear_links(&mut self) {
        self.details_url = None;
        self.state_url = None;
        self.large_url = None;
        self.small_url = None;
    }

    /// Drops what refers to a field that is gone: a link without its text or
    /// image, and a member list choice whose line is empty.
    pub fn drop_orphans(&mut self) {
        if self.details.is_none() {
            self.details_url = None;
        }
        if self.state.is_none() {
            self.state_url = None;
        }
        if self.large_image.is_none() {
            self.large_url = None;
        }
        if self.small_image.is_none() {
            self.small_url = None;
        }
        let empty = match self.status_display {
            Some(StatusDisplay::State) => self.state.is_none(),
            Some(StatusDisplay::Details) => self.details.is_none(),
            _ => false,
        };
        if empty {
            self.status_display = None;
        }
    }

    /// True when no text or image is left to publish.
    pub fn is_blank(&self) -> bool {
        self.state.is_none()
            && self.details.is_none()
            && self.large_image.is_none()
            && self.small_image.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(label: &str, url: &str) -> Button {
        Button {
            label: label.into(),
            url: url.into(),
        }
    }

    #[test]
    fn single_character_texts_are_dropped() {
        let activity = Activity {
            details: Some("A".into()),
            state: Some("Ok".into()),
            large_text: Some(String::new()),
            ..Activity::default()
        };
        let activity = activity.validated().unwrap();
        assert_eq!(activity.details, None);
        assert_eq!(activity.large_text, None);
        assert_eq!(activity.state.as_deref(), Some("Ok"));
    }

    #[test]
    fn long_texts_are_rejected() {
        let activity = Activity {
            state: Some("x".repeat(MAX_TEXT + 1)),
            ..Activity::default()
        };
        assert_eq!(
            activity.validated(),
            Err(ActivityError::TooLong {
                field: "state",
                limit: MAX_TEXT
            })
        );
    }

    #[test]
    fn length_counts_characters_not_bytes() {
        let activity = Activity {
            details: Some("é".repeat(MAX_TEXT)),
            ..Activity::default()
        };
        assert!(activity.validated().is_ok());
    }

    #[test]
    fn buttons_need_plain_web_links() {
        for url in [
            "javascript:alert(1)",
            "ftp://host/a",
            "https://user:pw@host/",
            "nothing",
        ] {
            let activity = Activity {
                buttons: vec![button("Open", url)],
                ..Activity::default()
            };
            assert_eq!(activity.validated(), Err(ActivityError::ButtonUrl), "{url}");
        }
        let good = Activity {
            buttons: vec![button("Open", "https://example.com/watch?v=1")],
            ..Activity::default()
        };
        assert!(good.validated().is_ok());
    }

    #[test]
    fn button_count_and_label_are_limited() {
        let many = Activity {
            buttons: vec![button("A1", "https://a.io"); 3],
            ..Activity::default()
        };
        assert_eq!(many.validated(), Err(ActivityError::TooManyButtons));
        let empty = Activity {
            buttons: vec![button("", "https://a.io")],
            ..Activity::default()
        };
        assert_eq!(empty.validated(), Err(ActivityError::ButtonLabel));
    }

    #[test]
    fn end_cannot_precede_start() {
        let activity = Activity {
            start: Some(10),
            end: Some(5),
            ..Activity::default()
        };
        assert_eq!(activity.validated(), Err(ActivityError::Timestamps));
    }

    #[test]
    fn links_on_texts_and_images_need_plain_web_addresses() {
        for url in [
            "javascript:alert(1)",
            "https://u:p@a.io/",
            "file:///c:/x",
            "a.io",
        ] {
            let activity = Activity {
                details: Some("Song".into()),
                details_url: Some(url.into()),
                ..Activity::default()
            };
            assert_eq!(
                activity.validated(),
                Err(ActivityError::Link {
                    field: "details_url"
                }),
                "{url}"
            );
        }
        let long = Activity {
            large_image: Some("cover".into()),
            large_url: Some(format!("https://a.io/{}", "x".repeat(MAX_BUTTON_URL))),
            ..Activity::default()
        };
        assert_eq!(
            long.validated(),
            Err(ActivityError::Link { field: "large_url" })
        );
        let good = Activity {
            state: Some("Artist".into()),
            state_url: Some("https://a.io/artist".into()),
            small_url: Some(String::new()),
            ..Activity::default()
        };
        let good = good.validated().unwrap();
        assert_eq!(good.state_url.as_deref(), Some("https://a.io/artist"));
        assert_eq!(good.small_url, None);
    }

    #[test]
    fn links_and_the_member_list_choice_need_their_field() {
        let activity = Activity {
            details: Some("A".into()),
            details_url: Some("https://a.io/1".into()),
            large_url: Some("https://a.io/2".into()),
            status_display: Some(StatusDisplay::Details),
            ..Activity::default()
        };
        // "A" is dropped as too short, and what pointed at it goes too.
        let activity = activity.validated().unwrap();
        assert_eq!(activity.details_url, None);
        assert_eq!(activity.large_url, None);
        assert_eq!(activity.status_display, None);
        let kept = Activity {
            state: Some("Artist".into()),
            status_display: Some(StatusDisplay::State),
            ..Activity::default()
        };
        assert_eq!(
            kept.validated().unwrap().status_display,
            Some(StatusDisplay::State)
        );
    }

    #[test]
    fn party_size_is_current_then_largest() {
        let with = |size| Activity {
            party_size: Some(size),
            ..Activity::default()
        };
        assert!(with([1, 1]).validated().is_ok());
        assert!(with([3, MAX_PARTY]).validated().is_ok());
        for bad in [[0, 4], [5, 4], [1, MAX_PARTY + 1]] {
            assert_eq!(with(bad).validated(), Err(ActivityError::Party), "{bad:?}");
        }
        let parsed: Activity = serde_json::from_str(r#"{"party_size":[2,4]}"#).unwrap();
        assert_eq!(parsed.party_size, Some([2, 4]));
        assert!(serde_json::from_str::<Activity>(r#"{"party_size":[2]}"#).is_err());
    }

    #[test]
    fn every_link_is_listed() {
        let activity = Activity {
            buttons: vec![button("Go", "https://b.io/")],
            details_url: Some("https://d.io/".into()),
            small_url: Some("https://s.io/".into()),
            ..Activity::default()
        };
        let links: Vec<&str> = activity.links().collect();
        assert_eq!(links, ["https://b.io/", "https://d.io/", "https://s.io/"]);
    }

    #[test]
    fn unknown_fields_are_refused_when_deserializing() {
        let parsed: Result<Activity, _> = serde_json::from_str(r#"{"details":"Hi","oops":1}"#);
        assert!(parsed.is_err());
    }
}
