/**
 * The tags a presence can carry. The list is closed so that one type of site always
 * gets the same word; crates/rpp-plugins/src/tags.rs holds the same words and a test
 * keeps them equal. Each one has a label in the locale files, under `tag.`.
 */
export const TAGS = [
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
] as const;

/** Most tags a presence can carry. */
export const MAX_TAGS = 8;
