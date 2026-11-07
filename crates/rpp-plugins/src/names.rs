//! What a presence name may be, and when two names are the same presence.

/// Longest accepted presence name, in characters.
pub const MAX_NAME: usize = 64;

/// Tells whether a name can be used for a presence.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME
        && name == name.trim()
        && !name.starts_with('.')
        && name.chars().any(char::is_alphanumeric)
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.'))
}

/// Longest name of a person, in characters.
pub const MAX_PERSON: usize = 64;

/// Tells whether a text can be shown as the name of an author or contributor.
pub fn valid_person(name: &str) -> bool {
    !name.trim().is_empty()
        && name.chars().count() <= MAX_PERSON
        && !name.chars().any(char::is_control)
}

/// The form of a name that decides whether two presences clash.
pub fn identity(name: &str) -> String {
    let mut key = String::with_capacity(name.len());
    for c in name.to_lowercase().chars() {
        if matches!(c, ' ' | '_' | '-' | '.') {
            if !key.ends_with('-') {
                key.push('-');
            }
        } else {
            key.push(c);
        }
    }
    key.trim_matches('-').to_owned()
}

/// Tells whether Windows treats a file or folder name as a device.
pub fn reserved_device(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    let stem = stem.to_ascii_lowercase();
    matches!(stem.as_str(), "con" | "prn" | "aux" | "nul")
        || ["com", "lpt"].iter().any(|prefix| {
            stem.strip_prefix(prefix)
                .is_some_and(|digit| matches!(digit.as_bytes(), [b'1'..=b'9']))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_one_rule() {
        for good in ["YouTube", "My Site", "a", "Café 2", "x.y_z-w", "日本語"] {
            assert!(valid_name(good), "{good}");
        }
        let long = "a".repeat(MAX_NAME + 1);
        for bad in [
            "", " ", " x", "x ", ".manual", "..", "- -", "___", "a/b", "a\\b", "a:b", "a\0b", &long,
        ] {
            assert!(!valid_name(bad), "{bad:?}");
        }
        assert!(valid_name(&"a".repeat(MAX_NAME)));
    }

    #[test]
    fn a_person_is_named_with_plain_text_of_up_to_64_characters() {
        assert!(valid_person("manucabral"));
        assert!(valid_person("Ana María"));
        assert!(valid_person(&"a".repeat(MAX_PERSON)));
        for bad in ["", "  ", "a\nb", "a\u{7}", &"a".repeat(MAX_PERSON + 1)] {
            assert!(!valid_person(bad), "{bad:?}");
        }
    }

    #[test]
    fn case_and_separators_do_not_make_a_different_presence() {
        let same = ["My Site", "my_site", "MY-SITE", "my.site", "my  -_ site"];
        for name in same {
            assert_eq!(identity(name), "my-site", "{name}");
        }
        assert_ne!(identity("My Site"), identity("MySite"));
        assert_eq!(identity("ÁRBOL"), identity("árbol"));
        assert_eq!(identity("--x--"), "x");
    }

    #[test]
    fn windows_device_names_are_found_whatever_the_dress() {
        for reserved in [
            "con", "CON", "nul", "Aux", "prn", "com1", "COM9", "lpt3", "con.js", "nul.txt", "Com1 ",
        ] {
            assert!(reserved_device(reserved), "{reserved}");
        }
        for fine in [
            "console",
            "com",
            "com0",
            "com10",
            "lpt",
            "connect.js",
            "main.js",
            "x.con",
            "",
            "icon",
        ] {
            assert!(!reserved_device(fine), "{fine}");
        }
    }
}
