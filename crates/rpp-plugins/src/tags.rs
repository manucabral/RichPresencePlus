//! The tags a presence can carry.

/// Every tag a presence can use, grouped by what someone is doing.
pub const TAGS: [&str; 26] = [
    "video",
    "streaming",
    "movie",
    "series",
    "anime",
    "live",
    "sport",
    "music",
    "radio",
    "podcast",
    "game",
    "multiplayer",
    "reading",
    "manga",
    "news",
    "wiki",
    "blog",
    "code",
    "design",
    "study",
    "social",
    "chat",
    "forum",
    "shopping",
    "ai",
    "tool",
];

/// True when a presence can use the word as a tag.
pub fn is_known_tag(tag: &str) -> bool {
    TAGS.contains(&tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_format_offers_the_same_words() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../rpp-app/templates/manifest.schema.json"))
                .unwrap();
        let words = schema["properties"]["tags"]["items"]["enum"]
            .as_array()
            .unwrap();
        let words: Vec<&str> = words.iter().filter_map(|word| word.as_str()).collect();
        assert_eq!(words, TAGS);
    }

    #[test]
    fn a_word_appears_once() {
        let mut sorted = TAGS;
        sorted.sort_unstable();
        assert!(sorted.windows(2).all(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn only_the_list_is_known() {
        assert!(is_known_tag("music"));
        assert!(!is_known_tag("Music"));
        assert!(!is_known_tag("invented"));
    }
}
