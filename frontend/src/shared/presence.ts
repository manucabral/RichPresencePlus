// Helpers for what a card shows about a presence: color, search and tags.

import type { GlobalPreferences, PresenceView } from "./types";

/** The fields a presence or catalog entry shares when it is listed. */
export interface Described {
  name: string;
  description: string | null;
  kind: string | null;
  author: string | null;
  tags: string[];
}

/** Groups the kind filter offers; anything else is "other". */
export const KINDS = ["game", "video", "music", "other"] as const;

/** Black or white, whichever reads better on a `#RRGGBB` background. */
export function readableOn(color: string): "#000000" | "#ffffff" {
  if (!/^#[0-9a-fA-F]{6}$/.test(color)) return "#ffffff";
  const value = Number.parseInt(color.slice(1), 16);
  const red = (value >> 16) & 255;
  const green = (value >> 8) & 255;
  const blue = value & 255;
  // Rec. 601 weights; the threshold favors white text on mid tones.
  return red * 0.299 + green * 0.587 + blue * 0.114 > 160 ? "#000000" : "#ffffff";
}

/** The first letter to draw when a presence has no logo. */
export function initial(name: string): string {
  return ([...name.trim()][0] ?? "?").toUpperCase();
}

/** Tells whether an item matches the search text and the chosen kind. */
export function matches(item: Described, text: string, kind: string): boolean {
  if (kind !== "" && (KINDS.includes(item.kind as (typeof KINDS)[number]) ? item.kind : "other") !== kind) {
    return false;
  }
  const query = text.trim().toLowerCase();
  if (query === "") return true;
  const haystack = [item.name, item.description ?? "", item.author ?? "", ...item.tags].join("\n").toLowerCase();
  return query.split(/\s+/).every((word) => haystack.includes(word));
}


/** The fields of an option that decide whether it is shown. */
export interface Conditional {
  key: string;
  type: string;
  value: unknown;
  if?: string;
}

/**
 * The options to draw: one whose `if` names a boolean option that is off is
 * hidden, and so is anything that depends on a hidden option. The order is kept.
 */
export function shownOptions<T extends Conditional>(options: readonly T[]): T[] {
  const byKey = new Map(options.map((option) => [option.key, option]));
  const shown = (option: T, depth: number): boolean => {
    if (!option.if) return true;
    const parent = byKey.get(option.if);
    // The backend refuses a missing or non-boolean target and loops; the depth
    // still keeps a bad answer from looping here.
    if (!parent || parent.type !== "boolean" || depth > options.length) return false;
    return parent.value === true && shown(parent, depth + 1);
  };
  return options.filter((option) => shown(option, 0));
}

/** Site patterns as people read them, each once: the host alone, or `*` for any site. */
export function siteNames(patterns: string[]): string[] {
  const names = patterns.map((pattern) =>
    pattern
      .replace(/^[^/]*:\/\//, "")
      .replace(/\/.*$/, "")
      .replace(/^\*\.(?=.)/, ""),
  );
  return [...new Set(names)];
}

/** What the Panel suggests while no presence has anything to show. */
export type IdleHint = { kind: "none" } | { kind: "play" } | { kind: "open"; names: string[] };

/** Names in the suggestion to open something; more would not read as a hint. */
const MOST_NAMED = 3;

/**
 * Says what would make an activity appear: turn one on, open a named one, or
 * play something. A presence that reads any site or the system player has no
 * one place to open.
 */
export function idleHint(presences: Pick<PresenceView, "name" | "enabled" | "access">[]): IdleHint {
  const enabled = presences.filter((presence) => presence.enabled);
  if (enabled.length === 0) return { kind: "none" };
  const named = enabled.filter(({ access }) => !access.media && !siteNames(access.sites).includes("*"));
  if (named.length === 0) return { kind: "play" };
  return { kind: "open", names: named.slice(0, MOST_NAMED).map((presence) => presence.name) };
}

/**
 * True when the privacy rules keep a presence from publishing at all: its
 * name is excluded, or every site it reads is.
 */
export function isExcluded(
  presence: Pick<PresenceView, "name" | "access">,
  rules: Pick<GlobalPreferences, "excluded_domains" | "excluded_applications">,
): boolean {
  if (rules.excluded_applications.includes(presence.name.toLowerCase())) return true;
  // The same comparison as `domain_excluded` in rpp-core: `www.` counts for nothing.
  const bare = (domain: string) => domain.replace(/^www\./, "");
  const sites = siteNames(presence.access.sites).map(bare);
  const excluded = rules.excluded_domains.map(bare);
  const blocked = (site: string) => excluded.some((domain) => site === domain || site.endsWith(`.${domain}`));
  return sites.length > 0 && sites.every(blocked);
}
