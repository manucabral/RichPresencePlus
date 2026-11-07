//! Translation tables: choosing a language and filling in values.

use std::collections::BTreeMap;

/// Keys and texts of one language.
pub type Table = BTreeMap<String, String>;

/// Language every table falls back to.
pub const BASE: &str = "en";

/// Normalizes a language tag such as `es_ar` or `EN` to `es-AR` or `en`.
///
/// Returns:
///     `None` when the tag is empty or has characters other than letters,
///     digits and separators.
pub fn normalize(tag: &str) -> Option<String> {
    let tag = tag.trim().split('.').next().unwrap_or_default();
    let mut parts = tag.split(['-', '_']);
    let language = parts.next().filter(|p| (2..=3).contains(&p.len()))?;
    if !language.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let mut out = language.to_ascii_lowercase();
    for part in parts {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_alphanumeric()) {
            return None;
        }
        out.push('-');
        if part.len() == 2 {
            out.push_str(&part.to_ascii_uppercase());
        } else {
            out.push_str(&part.to_ascii_lowercase());
        }
    }
    Some(out)
}

/// Picks the best available language for a request.
///
/// Returns:
///     A tag from `available`, or `None` if not even the base language exists.
pub fn pick(requested: &str, available: &[String]) -> Option<String> {
    let has = |tag: &str| available.iter().find(|a| a.as_str() == tag).cloned();
    if let Some(wanted) = normalize(requested) {
        if let Some(found) = has(&wanted) {
            return Some(found);
        }
        let language = wanted.split('-').next().unwrap_or_default();
        if let Some(found) = has(language) {
            return Some(found);
        }
        let prefix = format!("{language}-");
        if let Some(found) = available.iter().find(|a| a.starts_with(&prefix)) {
            return Some(found.clone());
        }
    }
    has(BASE)
}

/// The base table overlaid with the language closest to `requested`.
pub fn resolve(tables: &BTreeMap<String, Table>, requested: &str) -> Table {
    let mut merged = tables.get(BASE).cloned().unwrap_or_default();
    let available: Vec<String> = tables.keys().cloned().collect();
    if let Some(table) = pick(requested, &available)
        .filter(|tag| tag != BASE)
        .and_then(|tag| tables.get(&tag))
    {
        merged.extend(table.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    merged
}

/// Replaces `{name}` placeholders.
pub fn fill(text: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) if values.contains_key(&after[..close]) => {
                out.push_str(&values[&after[..close]]);
                rest = &after[close + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(pairs: &[(&str, &str)]) -> Table {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|t| (*t).to_owned()).collect()
    }

    #[test]
    fn tags_are_normalized() {
        assert_eq!(normalize("es_ar").as_deref(), Some("es-AR"));
        assert_eq!(normalize("EN").as_deref(), Some("en"));
        assert_eq!(normalize("zh-hans").as_deref(), Some("zh-hans"));
        assert_eq!(normalize("pt_BR.UTF-8").as_deref(), Some("pt-BR"));
        assert_eq!(normalize(""), None);
        assert_eq!(normalize("e"), None);
        assert_eq!(normalize("../x"), None);
        assert_eq!(normalize("es--AR"), None);
    }

    #[test]
    fn the_closest_language_is_picked() {
        let available = tags(&["en", "es", "pt-BR"]);
        assert_eq!(pick("es-AR", &available).as_deref(), Some("es"));
        assert_eq!(pick("pt", &available).as_deref(), Some("pt-BR"));
        assert_eq!(pick("pt-PT", &available).as_deref(), Some("pt-BR"));
        assert_eq!(pick("fr", &available).as_deref(), Some("en"));
        assert_eq!(pick("garbage!", &available).as_deref(), Some("en"));
        assert_eq!(pick("fr", &tags(&["es"])), None);
    }

    #[test]
    fn a_translation_falls_back_to_english_key_by_key() {
        let mut tables = BTreeMap::new();
        tables.insert("en".to_owned(), table(&[("a", "A"), ("b", "B")]));
        tables.insert("es".to_owned(), table(&[("a", "Uno")]));
        let spanish = resolve(&tables, "es-MX");
        assert_eq!(spanish["a"], "Uno");
        assert_eq!(spanish["b"], "B");
        assert_eq!(resolve(&tables, "de")["a"], "A");
    }

    #[test]
    fn placeholders_are_filled_and_unknown_ones_stay() {
        let values = table(&[("n", "3"), ("who", "Ana")]);
        assert_eq!(fill("{who} has {n} items", &values), "Ana has 3 items");
        assert_eq!(fill("{missing} and {n}", &values), "{missing} and 3");
        assert_eq!(fill("open { brace", &values), "open { brace");
        assert_eq!(fill("", &values), "");
    }
}
