//! Options a presence offers to the user.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// Most settings one presence may declare.
pub const MAX_SETTINGS: usize = 16;
/// Most choices in one `choice` setting.
const MAX_CHOICES: usize = 16;
/// Longest value of a `text` setting when the manifest gives no limit.
const DEFAULT_TEXT_LENGTH: usize = 64;
/// Longest value of a `text` setting a manifest may allow.
const MAX_TEXT_LENGTH: usize = 256;
/// Most settings one condition may look at.
pub const MAX_CONDITION_KEYS: usize = 4;

/// When a setting shows. Settings that do not show keep their value, and the
/// script still receives it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum When {
    /// The key of a boolean setting that must be on.
    On(String),
    /// Keys of other settings and the value each must have, or a list of
    /// values it may have. Every key must match.
    Values(BTreeMap<String, Value>),
}

impl When {
    /// Each setting this condition looks at, with the values that let it show.
    pub fn requirements(&self) -> Vec<(&str, Vec<Value>)> {
        match self {
            When::On(key) => vec![(key.as_str(), vec![Value::Bool(true)])],
            When::Values(values) => values
                .iter()
                .map(|(key, value)| {
                    let allowed = match value {
                        Value::Array(items) => items.clone(),
                        other => vec![other.clone()],
                    };
                    (key.as_str(), allowed)
                })
                .collect(),
        }
    }
}

/// One option of a presence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Setting {
    /// A switch.
    Boolean {
        /// Name the script reads it by.
        key: String,
        /// When it shows; always when omitted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<When>,
        /// Value until the user changes it.
        default: bool,
    },
    /// A number, optionally bounded.
    Number {
        /// Name the script reads it by.
        key: String,
        /// When it shows; always when omitted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<When>,
        /// Value until the user changes it.
        default: f64,
        /// Smallest accepted value.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        /// Largest accepted value.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    },
    /// A short text.
    Text {
        /// Name the script reads it by.
        key: String,
        /// When it shows; always when omitted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<When>,
        /// Value until the user changes it.
        default: String,
        /// Most characters accepted; 64 when omitted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_length: Option<usize>,
    },
    /// One of a fixed list.
    Choice {
        /// Name the script reads it by.
        key: String,
        /// When it shows; always when omitted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<When>,
        /// Choice used until the user changes it; one of `choices`.
        default: String,
        /// The values that can be picked.
        choices: Vec<String>,
    },
}

/// A key or choice: lower-case letters, digits, `_` and `-`, up to 32 characters.
fn plain(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 32
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
}

impl Setting {
    /// The name the script reads this setting by.
    pub fn key(&self) -> &str {
        match self {
            Setting::Boolean { key, .. }
            | Setting::Number { key, .. }
            | Setting::Text { key, .. }
            | Setting::Choice { key, .. } => key,
        }
    }

    /// When this setting shows, if it depends on others.
    pub fn condition(&self) -> Option<&When> {
        match self {
            Setting::Boolean { when, .. }
            | Setting::Number { when, .. }
            | Setting::Text { when, .. }
            | Setting::Choice { when, .. } => when.as_ref(),
        }
    }

    /// The value used until the user picks one.
    pub fn default_value(&self) -> Value {
        match self {
            Setting::Boolean { default, .. } => json!(default),
            Setting::Number { default, .. } => json!(default),
            Setting::Text { default, .. } | Setting::Choice { default, .. } => json!(default),
        }
    }

    /// Checks that the key, bounds, choices and default agree.
    ///
    /// Raises:
    ///     &'static str: A short reason for the first problem found.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !plain(self.key()) {
            return Err("setting keys use lower-case letters, digits, _ and -");
        }
        match self {
            Setting::Boolean { .. } => {}
            Setting::Number { min, max, .. } => {
                let finite = [min, max].into_iter().flatten().all(|n| n.is_finite());
                if !finite || min.zip(*max).is_some_and(|(low, high)| low > high) {
                    return Err("a number setting needs min not above max");
                }
            }
            Setting::Text { max_length, .. } => {
                if max_length.is_some_and(|n| n == 0 || n > MAX_TEXT_LENGTH) {
                    return Err("a text setting allows 1 to 256 characters");
                }
            }
            Setting::Choice { choices, .. } => {
                let unique: std::collections::HashSet<_> = choices.iter().collect();
                if choices.is_empty()
                    || choices.len() > MAX_CHOICES
                    || unique.len() != choices.len()
                    || !choices.iter().all(|choice| plain(choice))
                {
                    return Err("a choice setting needs 1 to 16 distinct plain choices");
                }
            }
        }
        // The default has to pass the same check as anything the user types.
        self.check(&self.default_value())
            .map(|_| ())
            .map_err(|_| "the default does not satisfy the setting's own limits")
    }

    /// Checks the user's value against this setting and returns it as stored.
    ///
    /// Raises:
    ///     String: Why the value is refused.
    pub fn check(&self, value: &Value) -> Result<Value, String> {
        let wrong = |what: &str| format!("{}: {what}", self.key());
        match self {
            Setting::Boolean { .. } => value
                .as_bool()
                .map(Value::Bool)
                .ok_or_else(|| wrong("expected true or false")),
            Setting::Number { min, max, .. } => {
                let number = value
                    .as_f64()
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| wrong("expected a number"))?;
                if min.is_some_and(|low| number < low) || max.is_some_and(|high| number > high) {
                    return Err(wrong("outside the allowed range"));
                }
                Ok(json!(number))
            }
            Setting::Text { max_length, .. } => {
                let text = value.as_str().ok_or_else(|| wrong("expected text"))?;
                let limit = max_length.unwrap_or(DEFAULT_TEXT_LENGTH);
                if text.chars().count() > limit || text.chars().any(char::is_control) {
                    return Err(wrong("too long or holds control characters"));
                }
                Ok(json!(text))
            }
            Setting::Choice { choices, .. } => value
                .as_str()
                .filter(|text| choices.iter().any(|choice| choice == text))
                .map(|text| json!(text))
                .ok_or_else(|| wrong("not one of the choices")),
        }
    }
}

/// Builds the complete set a script receives.
pub fn resolve(declared: &[Setting], saved: &BTreeMap<String, Value>) -> Map<String, Value> {
    declared
        .iter()
        .map(|setting| {
            let value = saved
                .get(setting.key())
                .and_then(|value| setting.check(value).ok())
                .unwrap_or_else(|| setting.default_value());
            (setting.key().to_owned(), value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: Value) -> Setting {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn every_kind_is_declared_with_its_own_fields() {
        let all = [
            json!({"type": "boolean", "key": "show", "default": true}),
            json!({"type": "number", "key": "size", "default": 5, "min": 1, "max": 10}),
            json!({"type": "text", "key": "label", "default": "Hi", "max_length": 10}),
            json!({"type": "choice", "key": "style", "default": "a", "choices": ["a", "b"]}),
        ];
        for value in all {
            let setting = parse(value.clone());
            assert!(setting.validate().is_ok(), "{value}");
            assert_eq!(serde_json::to_value(&setting).unwrap()["key"], value["key"]);
        }
        assert!(serde_json::from_value::<Setting>(json!({"type": "script", "key": "x"})).is_err());
        assert!(
            serde_json::from_value::<Setting>(
                json!({"type": "boolean", "key": "x", "default": true, "extra": 1})
            )
            .is_err()
        );
    }

    #[test]
    fn contradictory_declarations_are_refused() {
        let bad = [
            json!({"type": "boolean", "key": "Bad Key", "default": true}),
            json!({"type": "boolean", "key": "", "default": true}),
            json!({"type": "number", "key": "n", "default": 5, "min": 9, "max": 1}),
            json!({"type": "number", "key": "n", "default": 50, "max": 10}),
            json!({"type": "text", "key": "t", "default": "toolong", "max_length": 3}),
            json!({"type": "text", "key": "t", "default": "", "max_length": 0}),
            json!({"type": "text", "key": "t", "default": "", "max_length": 5000}),
            json!({"type": "choice", "key": "c", "default": "z", "choices": ["a", "b"]}),
            json!({"type": "choice", "key": "c", "default": "a", "choices": []}),
            json!({"type": "choice", "key": "c", "default": "a", "choices": ["a", "a"]}),
            json!({"type": "choice", "key": "c", "default": "A", "choices": ["A"]}),
        ];
        for value in bad {
            assert!(parse(value.clone()).validate().is_err(), "{value}");
        }
    }

    #[test]
    fn values_are_checked_by_kind() {
        let number =
            parse(json!({"type": "number", "key": "n", "default": 5, "min": 1, "max": 10}));
        assert_eq!(number.check(&json!(7)).unwrap(), json!(7.0));
        for bad in [
            json!(0),
            json!(11),
            json!("7"),
            json!(null),
            json!(f64::NAN),
        ] {
            assert!(number.check(&bad).is_err(), "{bad}");
        }
        let text = parse(json!({"type": "text", "key": "t", "default": "", "max_length": 4}));
        assert!(text.check(&json!("four")).is_ok());
        for bad in [json!("five!"), json!("a\nb"), json!(4)] {
            assert!(text.check(&bad).is_err(), "{bad}");
        }
        let choice =
            parse(json!({"type": "choice", "key": "c", "default": "a", "choices": ["a", "b"]}));
        assert!(choice.check(&json!("b")).is_ok());
        assert!(choice.check(&json!("c")).is_err());
        let switch = parse(json!({"type": "boolean", "key": "b", "default": false}));
        assert!(switch.check(&json!(true)).is_ok());
        assert!(switch.check(&json!("true")).is_err());
    }

    #[test]
    fn resolving_fills_defaults_and_drops_what_no_longer_fits() {
        let declared = vec![
            parse(json!({"type": "boolean", "key": "show", "default": true})),
            parse(json!({"type": "number", "key": "size", "default": 5, "max": 10})),
            parse(json!({"type": "choice", "key": "style", "default": "a", "choices": ["a", "b"]})),
        ];
        let saved = BTreeMap::from([
            ("show".to_owned(), json!(false)),
            // Out of range after an update: back to the default.
            ("size".to_owned(), json!(99)),
            // Declared by an older version only.
            ("gone".to_owned(), json!(1)),
        ]);
        let resolved = resolve(&declared, &saved);
        assert_eq!(resolved["show"], json!(false));
        assert_eq!(resolved["size"], json!(5.0));
        assert_eq!(resolved["style"], json!("a"));
        assert!(!resolved.contains_key("gone"));
        assert_eq!(resolve(&[], &saved).len(), 0);
    }
}
