// What the popup says. Pure functions, so they are tested without a browser.
import { isOpenRule } from "./pairing.js";

/**
 * Picks the message and the dot color for the background's status.
 *
 * @param {{connected: boolean, untrusted: boolean, paired: boolean, host: string}} state
 * @returns {{key: string, tone: "on" | "warn" | "bad" | ""}} A message key of
 *     _locales and the class of the status dot.
 */
export function describeState(state) {
  if (state.connected) return { key: "connected", tone: "on" };
  if (state.untrusted) return { key: "untrusted", tone: "bad" };
  if (state.paired) return { key: "waiting", tone: "warn" };
  if (state.host === "missing") return { key: "missingHost", tone: "bad" };
  return { key: "looking", tone: "" };
}

/**
 * The rule of the presence that reads a page, if any.
 *
 * @param {{presence: string, matches: string[]}[]} rules The rules from the app.
 * @param {string | undefined} url The page address; unknown without permission.
 * @param {(pattern: string, url: string) => boolean} matches Chrome's match test.
 * @returns {{presence: string, matches: string[]} | null}
 */
export function ruleForUrl(rules, url, matches) {
  if (!url) return null;
  const found = rules.filter((rule) => rule.matches.some((pattern) => matches(pattern, url)));
  // A presence made for this site goes before one for every site.
  return found.find((rule) => !isOpenRule(rule)) ?? found[0] ?? null;
}

/**
 * How long ago something started, as a message key and its number.
 *
 * @param {number} sinceMs When it started, in milliseconds.
 * @param {number} nowMs Now, in milliseconds.
 * @returns {{key: "forMoment" | "forMinutes" | "forHours", count: number}}
 */
export function elapsed(sinceMs, nowMs) {
  const minutes = Math.floor(Math.max(0, nowMs - sinceMs) / 60000);
  if (minutes < 1) return { key: "forMoment", count: 0 };
  if (minutes < 60) return { key: "forMinutes", count: minutes };
  return { key: "forHours", count: Math.floor(minutes / 60) };
}

/**
 * The presences the app asked this browser to read, once each and sorted.
 *
 * @param {{presence: string}[]} rules
 * @returns {string[]}
 */
export function presenceNames(rules) {
  return [...new Set(rules.map((rule) => rule.presence))].sort((a, b) => a.localeCompare(b));
}
