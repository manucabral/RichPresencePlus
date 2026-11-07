/**
 * Addresses named inside texts. The application opens them by name: the list of
 * real addresses is in src-tauri/src/commands.rs, so a text cannot open anything
 * else.
 */
const LINKS: Record<string, string> = {
  "discord.com/developers": "developers",
  "developer documentation": "docs",
  "documentación para desarrolladores": "docs-es",
  "publishing guide": "publish",
  "guía de publicación": "publish-es",
  "github.com/manucabral/RichPresencePlus/issues": "issues",
};

const FOUND = new RegExp(`(${Object.keys(LINKS).join("|").replaceAll(".", "\\.")})`);

export interface TextPart {
  text: string;
  /** What to open, when this part is a link. */
  target: string | null;
}

/** Splits a text into plain parts and the links it names. */
export function linkParts(text: string): TextPart[] {
  return text
    .split(FOUND)
    .filter((part) => part !== "")
    .map((part) => ({ text: part, target: LINKS[part] ?? null }));
}
