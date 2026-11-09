// What every presence script can use. Your editor reads this file to autocomplete and to
// explain each call; the application does not load it. `presence-types.d.ts`, next to
// it, holds what belongs to this presence alone: its values, its options and `host`.

/** One page of the browser that matches this presence. */
interface Page {
  /** Full address of the page. */
  url: string;
  /** Title of the tab. */
  title: string;
  /** When the page was read, in Unix milliseconds. Pass it to `playback`. */
  received_at: number;
  /** What the manifest asks to read, under `page.extract` and `page.frames.extract`. */
  values: Values;
}

/** What the application gives to `read` on each call. */
interface Context {
  /** The time, in Unix seconds. Use it for `start`, and to remember when something began. */
  now: number;
  /** Pages open for this presence. Empty for a desktop presence or with no page open. */
  pages: Page[];
  /** The value of each option of the manifest, or its default. */
  options: Options;
}

/** The card Discord shows. Every field is optional, but it needs a text or an image. */
interface Activity {
  /** The title, in place of the Discord application's name. */
  name?: string | null;
  /** First line of text, up to 128 characters. */
  details?: string | null;
  /** Second line of text, up to 128 characters. */
  state?: string | null;
  /** How Discord words the activity: "Watching", "Playing"... */
  kind?: "playing" | "listening" | "watching" | "competing";
  /** Unix seconds. Alone it counts up; with `end` it draws a progress bar. */
  start?: number | null;
  /** Unix seconds when it ends. */
  end?: number | null;
  /** An https address, or the name of an image of the Discord application. */
  large_image?: string | null;
  /** Text shown when hovering the large image. */
  large_text?: string | null;
  /** Small picture in the corner of the large one. */
  small_image?: string | null;
  /** Text shown when hovering the small image. */
  small_text?: string | null;
  /** Up to two buttons. */
  buttons?: { label: string; url: string }[];
  /** Address opened when clicking the first line. */
  details_url?: string | null;
  /** Address opened when clicking the second line. */
  state_url?: string | null;
  /** Address opened when clicking the large image. */
  large_url?: string | null;
  /** Address opened when clicking the small image. */
  small_url?: string | null;
  /** `[current, largest]`, shown as "(2 of 4)". */
  party_size?: [number, number] | null;
  /** Which text the member list shows next to the person's name. */
  status_display?: "name" | "state" | "details" | null;
}

/** Facts Discord does not show. They help choose between presences. */
interface Meta {
  /** A paused presence gives way to one that plays, and hides by default. */
  paused?: boolean;
  /** Ranks the presence higher while its window has focus. */
  foreground?: boolean;
  /** Program name, so people can exclude it in Privacy. */
  application?: string;
  /** Extra sites, so people can exclude them in Privacy. */
  domains?: string[];
}

/** The value of a `media` extractor. */
interface Player {
  paused: boolean;
  /** Position in seconds. */
  current: number;
  /** Length in seconds, or null while it is unknown. */
  duration: number | null;
  /** Playback speed: 1 is normal. */
  rate: number;
}

/** The value of a `session` extractor. */
interface Session {
  title: string | null;
  artist: string | null;
  album: string | null;
  artwork: string | null;
  state: string | null;
}

/**
 * The text of `key` in the person's language, from the `locales` folder.
 * @param key A key of `locales/en.json`.
 * @param values What fills `{name}` placeholders.
 * @example t("by", { name: "Manu" }) // "By Manu"
 */
declare function t(key: string, values?: Record<string, unknown>): string;

/**
 * Shortens a text so Discord accepts it.
 * @param text The text. Anything else gives null.
 * @param max Most characters; 128 by default.
 */
declare function cut(text: unknown, max?: number): string | null;

/**
 * Splits an address. There is no `URL` class in a presence.
 * @example parseUrl("https://a.com/watch?v=1").query.v // "1"
 */
declare function parseUrl(address: string): { host: string; path: string; query: Record<string, string> } | null;

/**
 * Chooses when several tabs match: the one playing, then one with a player, then the
 * first. It looks for a value named `player`.
 */
declare function bestPage(pages: Page[]): Page | null;

/**
 * The playing flag and the progress bar for a `media` value.
 * @param player The value of the extractor.
 * @param receivedAt `page.received_at`.
 * @example return { title, ...playback(page.values.player, page.received_at) };
 */
declare function playback(player: Player | null, receivedAt: number): { playing: boolean; start: number | null; end: number | null };

/**
 * Reads a clock text as seconds.
 * @example parseTime("1:02:03") // 3723
 */
declare function parseTime(text: unknown): number | null;

/**
 * The progress bar for a position and a length.
 * @param current Position in seconds.
 * @param duration Length in seconds.
 * @param now `context.now`.
 */
declare function progress(current: number, duration: number, now: number): { start: number; end: number };

/**
 * One of the items, the next one every `seconds` seconds. Empty items are skipped.
 * @param seconds How long each one stays; 30 when it is less than 1.
 * @param now `context.now`.
 * @example rotate([video.title, video.channel], 30, context.now)
 */
declare function rotate<T>(items: (T | null | undefined)[], seconds: number, now: number): T | null;

/**
 * Fills a text the person wrote, such as an option, with `{name}` placeholders. A
 * missing value leaves nothing, and a separator left at either end goes with it.
 * @example format("{title} - {channel}", { title: "A walk", channel: null }) // "A walk"
 */
declare function format(template: unknown, values: Record<string, unknown>): string | null;

/**
 * The address of a small picture every presence can use, for `small_image`.
 * @example small_image: icon(video.playing ? "play" : "pause")
 */
declare function icon(name: "play" | "pause" | "stop" | "live" | "repeat" | "repeat-one" | "shuffle"): string;

/** `host.window`, with the `window` permission. */
interface HostWindow {
  /** The window in front. Its title often holds a file or project name. */
  foreground(): { process: string; title: string } | null;
}

/** `host.media`, with the `media` permission. */
interface HostMedia {
  /** What Windows says is playing. */
  current(): {
    app: string;
    title: string;
    artist: string;
    album: string;
    status: string;
    position: number;
    duration: number;
  } | null;
}

/** `host.http`, with the `http` permission. */
interface HostHttp {
  /** Fetches an https address of the hosts the manifest lists. It follows no redirects. */
  get(url: string): { status: number; body: string };
}

/** `host.files`, with the `files` permission. */
interface HostFiles {
  /** The folders the manifest lists. */
  roots: string[];
  /** The newest file of a folder that ends in `suffix`. */
  latest(dir: string, suffix: string): string | null;
  /** The last lines of a file. */
  tail(path: string, lines: number): string[];
}

/** `host.query`, with the `query` permission: what a game server says about itself. */
type HostQuery = (protocol: string, address: string, port: number) => Record<string, any> | null;
