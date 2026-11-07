// What the interface says about the connected browsers and about Discord.

/** Browsers the bridge names; anything else arrives as "other". */
export type BrowserKind = "chrome" | "edge" | "firefox" | "opera" | "brave" | "vivaldi" | "yandex" | "other";

type Translate = (key: string, values?: Record<string, string | number>) => string;

/** Short names, for lists where space is tight. */
const SHORT: Record<string, string> = {
  chrome: "Chrome",
  edge: "Edge",
  firefox: "Firefox",
  opera: "Opera",
  brave: "Brave",
  vivaldi: "Vivaldi",
  yandex: "Yandex",
};

/** Names as each maker writes them. */
const FULL: Record<string, string> = {
  chrome: "Google Chrome",
  edge: "Microsoft Edge",
  firefox: "Mozilla Firefox",
  opera: "Opera",
  brave: "Brave",
  vivaldi: "Vivaldi",
  yandex: "Yandex Browser",
};

// hasOwn, not `in`: a name like "toString" must not reach the prototype.
function lookup(table: Record<string, string>, browser: string, t: Translate): string {
  return Object.hasOwn(table, browser) ? table[browser] : t("browser.other");
}

/** The full name of a browser the bridge reports. */
export function browserName(browser: string, t: Translate): string {
  return lookup(FULL, browser, t);
}

/**
 * Names the connected browsers once each, with a count for repeats, such as
 * "Chrome, Edge (2)". Empty when none is connected.
 */
export function browserList(browsers: readonly string[], t: Translate): string {
  const counts = new Map<string, number>();
  for (const browser of browsers) {
    const name = lookup(SHORT, browser, t);
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  return [...counts].map(([name, count]) => (count > 1 ? `${name} (${count})` : name)).join(", ");
}

/**
 * A length of time in words: under a minute, minutes, or hours and minutes.
 * Negative seconds count as zero.
 */
export function duration(seconds: number, t: Translate): string {
  const minutes = Math.floor(Math.max(0, seconds) / 60);
  if (minutes < 1) return t("time.underMinute");
  if (minutes < 60) return t("time.minutes", { minutes });
  return t("time.hours", { hours: Math.floor(minutes / 60), minutes: String(minutes % 60).padStart(2, "0") });
}

/** The line under the title of the Discord card. */
export function discordText(running: boolean, user: string | null, t: Translate): string {
  if (!running) return t("dashboard.notRunning");
  return user ? t("dashboard.publishingAs", { name: user }) : t("dashboard.running");
}

/** Advice shown above the status when Discord is not running; null when it is. */
export function discordAdvice(running: boolean, t: Translate): string | null {
  return running ? null : t("dashboard.openDiscord");
}
