//! How many times each presence was installed, as the catalog publishes it
//! once a day.
//!
//! The file is not signed. Its numbers are only shown next to a presence and
//! never decide what is installed, so anything malformed is left out.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::index::{Channel, MAX_PRESENCES, plain_folder};

/// Largest counts file read, in bytes.
pub const MAX_INSTALLS: u64 = 1024 * 1024;

/// Installs of a channel by presence folder, from the bytes of `stats.json`.
///
/// Returns:
///     Only folders with a plain name and a count above zero, at most as many
///     as an index can list. Empty for a file that is too long or not JSON.
pub fn parse(bytes: &[u8], channel: Channel) -> BTreeMap<String, u64> {
    let mut installs = BTreeMap::new();
    if bytes.len() as u64 > MAX_INSTALLS {
        return installs;
    }
    let key = match channel {
        Channel::Stable => "stable",
        Channel::Experimental => "experimental",
    };
    let Ok(stats) = serde_json::from_slice::<Value>(bytes) else {
        return installs;
    };
    let rows = stats.get(key).and_then(Value::as_array);
    for row in rows.into_iter().flatten() {
        if installs.len() >= MAX_PRESENCES {
            break;
        }
        let folder = row.get("folder").and_then(Value::as_str);
        let count = row.get("installs").and_then(Value::as_u64);
        if let (Some(folder), Some(count)) = (folder, count)
            && plain_folder(folder)
            && count > 0
        {
            installs.entry(folder.to_owned()).or_insert(count);
        }
    }
    installs
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bytes(value: Value) -> Vec<u8> {
        value.to_string().into_bytes()
    }

    #[test]
    fn counts_are_read_for_the_channel_asked() {
        let stats = bytes(json!({
            "generated_at": 1,
            "stable": [{"folder": "youtube", "installs": 12}, {"folder": "roblox", "installs": 3}],
            "experimental": [{"folder": "youtube", "installs": 1}],
        }));
        let stable = parse(&stats, Channel::Stable);
        assert_eq!(stable.len(), 2);
        assert_eq!(stable["youtube"], 12);
        assert_eq!(parse(&stats, Channel::Experimental)["youtube"], 1);
    }

    #[test]
    fn anything_malformed_is_left_out() {
        let stats = bytes(json!({"stable": [
            {"folder": "good", "installs": 2},
            {"folder": "../up", "installs": 5},
            {"folder": "Upper", "installs": 5},
            {"folder": "zero", "installs": 0},
            {"folder": "negative", "installs": -4},
            {"folder": "fraction", "installs": 1.5},
            {"folder": "text", "installs": "9"},
            {"folder": 7, "installs": 9},
            {"installs": 9},
            "row",
            null,
            // The first count of a folder stays.
            {"folder": "good", "installs": 900},
        ]}));
        let installs = parse(&stats, Channel::Stable);
        assert_eq!(installs.len(), 1);
        assert_eq!(installs["good"], 2);

        for other in [
            &b"not json"[..],
            b"",
            b"[]",
            b"{\"stable\": {\"folder\": \"a\"}}",
            b"{\"experimental\": []}",
        ] {
            assert!(parse(other, Channel::Stable).is_empty());
        }
    }

    #[test]
    fn a_file_that_is_too_long_or_lists_too_much_is_cut() {
        let long = vec![b' '; MAX_INSTALLS as usize + 1];
        assert!(parse(&long, Channel::Stable).is_empty());

        let rows: Vec<Value> = (0..MAX_PRESENCES + 10)
            .map(|n| json!({"folder": format!("p{n}"), "installs": 1}))
            .collect();
        let many = parse(&bytes(json!({"stable": rows})), Channel::Stable);
        assert_eq!(many.len(), MAX_PRESENCES);
    }
}
