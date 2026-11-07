//! Options read from another program's command line.

use serde_json::{Map, Value};

/// Longest value returned, in characters.
pub const MAX_VALUE: usize = 256;

/// Splits a Windows command line into arguments, dropping the program path.
pub fn split_command_line(command_line: &str) -> Vec<String> {
    let chars: Vec<char> = command_line.chars().collect();
    let mut index = 0;
    while index < chars.len() && chars[index].is_whitespace() {
        index += 1;
    }
    // The program path ends at the first unquoted space; backslashes are literal.
    let mut in_quotes = false;
    while index < chars.len() {
        let ch = chars[index];
        index += 1;
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes && ch.is_whitespace() {
            break;
        }
    }

    let mut args = Vec::new();
    loop {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index >= chars.len() {
            break;
        }
        let mut current = String::new();
        let mut quoted = false;
        while index < chars.len() {
            let mut backslashes = 0;
            while index < chars.len() && chars[index] == '\\' {
                backslashes += 1;
                index += 1;
            }
            if index < chars.len() && chars[index] == '"' {
                current.extend(std::iter::repeat_n('\\', backslashes / 2));
                if backslashes % 2 == 0 {
                    if quoted && index + 1 < chars.len() && chars[index + 1] == '"' {
                        current.push('"');
                        index += 1;
                    } else {
                        quoted = !quoted;
                    }
                } else {
                    current.push('"');
                }
                index += 1;
            } else {
                current.extend(std::iter::repeat_n('\\', backslashes));
                if index >= chars.len() || (!quoted && chars[index].is_whitespace()) {
                    break;
                }
                current.push(chars[index]);
                index += 1;
            }
        }
        args.push(current);
    }
    args
}

/// Tells whether an argument is an option rather than a value: `-x`, `--x`
/// or `/x` with a letter. A negative number is a value.
fn looks_like_option(arg: &str) -> bool {
    let rest = arg.trim_start_matches(['-', '/']);
    rest.len() < arg.len() && rest.chars().next().is_some_and(char::is_alphabetic)
}

/// A value fit to return: short, without control characters.
fn clean(value: &str) -> Option<Value> {
    let value = value.trim();
    let fits = value.chars().count() <= MAX_VALUE && !value.chars().any(char::is_control);
    fits.then(|| Value::from(value))
}

/// Picks the declared options out of an argument list.
///
/// Returns:
///     One entry per declared option found, keyed as the manifest writes it.
///     A value that is too long or holds control characters is left out.
pub fn pick(args: &[String], flags: &[String]) -> Map<String, Value> {
    let mut found = Map::new();
    for (index, arg) in args.iter().enumerate() {
        for flag in flags {
            if found.contains_key(flag) {
                continue;
            }
            let value = if arg == flag {
                match args.get(index + 1) {
                    Some(next) if !looks_like_option(next) => clean(next),
                    _ => Some(Value::Bool(true)),
                }
            } else {
                let joined = arg
                    .strip_prefix(flag.as_str())
                    .and_then(|rest| rest.strip_prefix('='));
                match joined {
                    Some(value) => clean(value),
                    None => continue,
                }
            };
            if let Some(value) = value {
                found.insert(flag.clone(), value);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| (*a).to_owned()).collect()
    }

    fn flags(list: &[&str]) -> Vec<String> {
        args(list)
    }

    #[test]
    fn command_lines_split_like_the_c_runtime() {
        assert_eq!(
            split_command_line(
                r#""C:\Games\GTA San Andreas\gta_sa.exe" -c -h 51.254.85.134 -p 7777 -n Nick"#
            ),
            ["-c", "-h", "51.254.85.134", "-p", "7777", "-n", "Nick"]
        );
        assert_eq!(
            split_command_line(r#"gta_sa.exe -n "Two Words" -z "a\"b" C:\dir\"#),
            ["-n", "Two Words", "-z", "a\"b", r"C:\dir\"]
        );
        assert!(split_command_line(r#""C:\Games\gta_sa.exe""#).is_empty());
        assert!(split_command_line("   ").is_empty());
    }

    #[test]
    fn only_declared_options_are_returned() {
        let line = args(&[
            "-c", "-n", "Nick", "-h", "1.2.3.4", "-p", "7777", "-z", "secret",
        ]);
        let picked = pick(&line, &flags(&["-h", "-p"]));
        assert_eq!(
            Value::Object(picked),
            json!({"-h": "1.2.3.4", "-p": "7777"})
        );
        let text = serde_json::to_string(&pick(&line, &flags(&["-h", "-p", "-c"]))).unwrap();
        assert!(!text.contains("secret") && !text.contains("Nick"), "{text}");
    }

    #[test]
    fn options_take_a_separate_or_joined_value_or_none() {
        let line = args(&["--port=7777", "-c", "-h", "-p", "-5", "/server", "eu"]);
        let picked = pick(
            &line,
            &flags(&["--port", "-c", "-h", "-p", "/server", "-x"]),
        );
        assert_eq!(
            Value::Object(picked),
            json!({"--port": "7777", "-c": true, "-h": true, "-p": "-5", "/server": "eu"})
        );
        // The first occurrence wins.
        let again = pick(&args(&["-h", "a", "-h", "b"]), &flags(&["-h"]));
        assert_eq!(Value::Object(again), json!({"-h": "a"}));
        // A name in other case is another option, which was not declared.
        let other = pick(&args(&["-P", "secret", "-P=secret"]), &flags(&["-p"]));
        assert!(other.is_empty());
        // `-hp` is not `-h` followed by `p`.
        assert!(pick(&args(&["-hp"]), &flags(&["-h"])).is_empty());
    }

    #[test]
    fn values_that_are_too_long_or_hold_control_characters_are_left_out() {
        let long = "x".repeat(MAX_VALUE + 1);
        let line = args(&["-a", &long, "-b", "bad\u{7}bell", "-c", " ok "]);
        let picked = pick(&line, &flags(&["-a", "-b", "-c"]));
        assert_eq!(Value::Object(picked), json!({"-c": "ok"}));
    }
}
