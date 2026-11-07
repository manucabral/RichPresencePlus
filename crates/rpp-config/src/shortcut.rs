//! The keyboard shortcut that pauses publishing from anywhere.

/// Modifiers in the order they are written.
const MODIFIERS: [&str; 4] = ["Ctrl", "Alt", "Shift", "Super"];

fn modifier(word: &str) -> Option<&'static str> {
    match word.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some("Ctrl"),
        "alt" | "option" => Some("Alt"),
        "shift" => Some("Shift"),
        "super" | "win" | "windows" | "meta" | "cmd" => Some("Super"),
        _ => None,
    }
}

fn key(word: &str) -> Option<String> {
    let upper = word.to_ascii_uppercase();
    let single = upper.len() == 1 && upper.bytes().all(|b| b.is_ascii_alphanumeric());
    let function = upper
        .strip_prefix('F')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=24).contains(&n));
    (single || function).then_some(upper)
}

/// Checks a shortcut such as `ctrl+alt+p` and writes it as stored: `Ctrl+Alt+P`,
/// modifiers in a fixed order.
///
/// Raises:
///     String: A word is unknown, a modifier repeats, there is not exactly
///         one key, or no modifier other than Shift.
pub fn normalize(text: &str) -> Result<String, String> {
    let mut found: Vec<&'static str> = Vec::new();
    let mut keys = Vec::new();
    for word in text.split('+').map(str::trim) {
        if let Some(name) = modifier(word) {
            if found.contains(&name) {
                return Err(format!("{name} appears twice."));
            }
            found.push(name);
        } else if let Some(name) = key(word) {
            keys.push(name);
        } else {
            return Err(format!("\"{word}\" is not a key the shortcut can use."));
        }
    }
    if keys.len() != 1 {
        return Err("Use one key: a letter, a digit or F1 to F24.".into());
    }
    if !found.iter().any(|name| *name != "Shift") {
        return Err("Add Ctrl, Alt or the Windows key.".into());
    }
    let mut parts: Vec<String> = MODIFIERS
        .iter()
        .filter(|name| found.contains(name))
        .map(|name| (*name).to_owned())
        .collect();
    parts.push(keys.remove(0));
    Ok(parts.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_are_written_in_one_order() {
        assert_eq!(normalize("alt+ctrl+p").unwrap(), "Ctrl+Alt+P");
        assert_eq!(normalize(" Win + Shift + f12 ").unwrap(), "Shift+Super+F12");
        assert_eq!(normalize("control+9").unwrap(), "Ctrl+9");
    }

    #[test]
    fn shortcuts_that_would_take_plain_typing_are_refused() {
        for bad in [
            "",
            "P",
            "Shift+P",
            "Ctrl",
            "Ctrl+P+Q",
            "Ctrl+Ctrl+P",
            "Ctrl+Space",
            "Ctrl+F25",
            "Ctrl+F0",
            "Ctrl+é",
            "Ctrl+<script>",
        ] {
            assert!(normalize(bad).is_err(), "{bad:?}");
        }
    }
}
