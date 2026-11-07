//! Manifests come from anyone who writes a presence.

use proptest::prelude::*;
use rpp_plugins::Manifest;
use serde_json::json;

proptest! {
    #[test]
    fn any_text_gives_a_manifest_or_an_error(text in "\\PC{0,400}") {
        let _ = Manifest::parse(&text);
    }

    #[test]
    fn odd_values_in_the_right_shape_are_safe(
        name in "\\PC{0,80}",
        client_id in "\\PC{0,40}",
        entry in "[a-zA-Z0-9./\\\\:]{0,40}",
        interval in any::<i64>(),
        color in "\\PC{0,12}",
    ) {
        let manifest = json!({
            "name": name,
            "version": "1.0.0",
            "client_id": client_id,
            "entry": entry,
            "interval": interval,
            "color": color,
        });
        let _ = Manifest::parse(&manifest.to_string());
    }

    #[test]
    fn an_entry_that_climbs_out_is_refused(depth in 1_usize..6, file in "[a-z]{1,8}") {
        let manifest = json!({
            "name": "Demo",
            "version": "1.0.0",
            "client_id": "12345678901234567",
            "entry": format!("{}{file}.js", "../".repeat(depth)),
        });
        prop_assert!(Manifest::parse(&manifest.to_string()).is_err());
    }
}
