//! The JSON Discord expects for the commands this application sends.

use rpp_core::Activity;
use serde_json::{Map, Value, json};

fn insert_text(object: &mut Map<String, Value>, key: &str, value: &Option<String>) {
    if let Some(text) = value {
        object.insert(key.into(), json!(text));
    }
}

/// The activity object Discord expects, with only the fields that are present.
///
/// Args:
///     party: Identifier of the group, sent with `party_size`. Discord groups
///         users whose activities share it, so each connection makes its own.
pub fn activity_json(activity: &Activity, party: &str) -> Value {
    let mut object = Map::new();
    insert_text(&mut object, "name", &activity.name);
    insert_text(&mut object, "state", &activity.state);
    insert_text(&mut object, "state_url", &activity.state_url);
    insert_text(&mut object, "details", &activity.details);
    insert_text(&mut object, "details_url", &activity.details_url);
    object.insert("type".into(), json!(activity.kind.code()));
    if let Some(display) = activity.status_display {
        object.insert("status_display_type".into(), json!(display.code()));
    }
    if let Some(size) = activity.party_size {
        object.insert("party".into(), json!({"id": party, "size": size}));
    }

    let mut timestamps = Map::new();
    if let Some(start) = activity.start {
        timestamps.insert("start".into(), json!(start));
    }
    if let Some(end) = activity.end {
        timestamps.insert("end".into(), json!(end));
    }
    if !timestamps.is_empty() {
        object.insert("timestamps".into(), Value::Object(timestamps));
    }

    let mut assets = Map::new();
    insert_text(&mut assets, "large_image", &activity.large_image);
    insert_text(&mut assets, "large_text", &activity.large_text);
    insert_text(&mut assets, "large_url", &activity.large_url);
    insert_text(&mut assets, "small_image", &activity.small_image);
    insert_text(&mut assets, "small_text", &activity.small_text);
    insert_text(&mut assets, "small_url", &activity.small_url);
    if !assets.is_empty() {
        object.insert("assets".into(), Value::Object(assets));
    }
    if !activity.buttons.is_empty() {
        object.insert("buttons".into(), json!(activity.buttons));
    }
    Value::Object(object)
}

/// The `SET_ACTIVITY` command; `None` clears the activity.
///
/// Args:
///     pid: Process id shown to Discord as the owner of the activity.
///     nonce: Identifier echoed in Discord's answer.
///     party: Identifier of the group, see [`activity_json`].
pub fn set_activity(pid: u32, activity: Option<&Activity>, nonce: &str, party: &str) -> Value {
    json!({
        "cmd": "SET_ACTIVITY",
        "args": {"pid": pid, "activity": activity.map(|a| activity_json(a, party))},
        "nonce": nonce,
    })
}

/// The payload of the handshake frame.
pub fn handshake(client_id: &str) -> Value {
    json!({"v": 1, "client_id": client_id})
}

#[cfg(test)]
mod tests {
    use super::*;
    use rpp_core::{ActivityKind, Button, StatusDisplay};

    #[test]
    fn only_present_fields_are_sent() {
        let activity = Activity {
            details: Some("Song".into()),
            kind: ActivityKind::Listening,
            start: Some(10),
            large_image: Some("cover".into()),
            buttons: vec![Button {
                label: "Open".into(),
                url: "https://a.io".into(),
            }],
            ..Activity::default()
        };
        assert_eq!(
            activity_json(&activity, "p"),
            json!({
                "details": "Song",
                "type": 2,
                "timestamps": {"start": 10},
                "assets": {"large_image": "cover"},
                "buttons": [{"label": "Open", "url": "https://a.io"}],
            })
        );
    }

    #[test]
    fn newer_fields_go_where_discord_reads_them() {
        let activity = Activity {
            details: Some("Song".into()),
            details_url: Some("https://a.io/s".into()),
            state: Some("Artist".into()),
            state_url: Some("https://a.io/a".into()),
            large_image: Some("cover".into()),
            large_url: Some("https://a.io/c".into()),
            small_image: Some("logo".into()),
            small_url: Some("https://a.io/l".into()),
            status_display: Some(StatusDisplay::State),
            party_size: Some([2, 4]),
            ..Activity::default()
        };
        let json = activity_json(&activity, "party-1");
        assert_eq!(json["details_url"], "https://a.io/s");
        assert_eq!(json["state_url"], "https://a.io/a");
        assert_eq!(json["assets"]["large_url"], "https://a.io/c");
        assert_eq!(json["assets"]["small_url"], "https://a.io/l");
        assert_eq!(json["status_display_type"], 1);
        assert_eq!(json["party"], json!({"id": "party-1", "size": [2, 4]}));
        // Without a size there is no party, so no identifier leaves.
        let plain = activity_json(&Activity::default(), "party-1");
        assert!(plain.get("party").is_none());
        assert!(plain.get("status_display_type").is_none());
    }

    #[test]
    fn clearing_sends_a_null_activity() {
        let command = set_activity(7, None, "n", "p");
        assert_eq!(command["args"]["activity"], Value::Null);
        assert_eq!(command["args"]["pid"], 7);
    }

    /// Checked against the real Discord on 2026-11-05: it accepts the name and answers
    /// with it, on an application shared by several presences.
    #[test]
    fn the_name_of_an_activity_goes_with_it() {
        let named = Activity {
            name: Some("My Site".into()),
            details: Some("Reading".into()),
            ..Activity::default()
        };
        assert_eq!(activity_json(&named, "party")["name"], "My Site");
        let plain = activity_json(&Activity::default(), "party");
        assert!(
            plain.get("name").is_none(),
            "the application's name stays when there is none"
        );
    }
}
