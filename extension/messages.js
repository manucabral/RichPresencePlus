// Who may send which message to the service worker. Pure, so it is tested
// without a browser.
//
// Page scripts run inside web pages: they may report pages, ask for rules and
// end a pick the popup started, nothing else. Pairing, pausing and reading the
// status belong to the extension's own pages, such as the popup. Web pages
// themselves cannot send
// messages at all: the manifest has no `externally_connectable`.

/** Messages page scripts send from a tab. */
export const PAGE_MESSAGES = new Set([
  "rules",
  "page",
  "left",
  "pick-done",
  "pick-cancel",
  "frame-rules",
  "frame",
  "frame-left",
]);
/** Messages only the extension's own pages send. */
export const POPUP_MESSAGES = new Set(["status", "pair", "refresh", "tab", "pause", "open-site", "pick-start"]);

/**
 * Tells whether a sender may send a message.
 *
 * @param {unknown} message What arrived.
 * @param {{id?: string, tab?: object, url?: string}} sender Who sent it, as the browser says.
 * @param {{id: string, base: string}} self This extension's id and the start of its page addresses.
 * @returns {boolean}
 */
export function mayReceive(message, sender, self) {
  if (!message || typeof message !== "object" || typeof message.type !== "string") return false;
  if (!sender || sender.id !== self.id) return false;
  if (PAGE_MESSAGES.has(message.type)) return Boolean(sender.tab);
  if (POPUP_MESSAGES.has(message.type)) {
    return !sender.tab && typeof sender.url === "string" && sender.url.startsWith(self.base);
  }
  return false;
}

/** Largest page report. Sealed in base64 it stays under the 256 KiB the app accepts. */
export const MAX_PAGE_BYTES = 150 * 1024;

/**
 * Tells whether a page report can travel on the link to the app.
 *
 * @param {object} page
 * @returns {boolean}
 */
export function fitsLink(page) {
  return new TextEncoder().encode(JSON.stringify(page)).length <= MAX_PAGE_BYTES;
}

/**
 * Takes the site of a page a presence can be made for.
 *
 * @param {string} [url] Address of the page.
 * @returns {string|null} Its host in lowercase, or null for anything but a web page.
 */
export function pageHost(url) {
  try {
    const { protocol, hostname } = new URL(url);
    return protocol === "http:" || protocol === "https:" ? hostname.toLowerCase() : null;
  } catch {
    return null;
  }
}

/** Longest name and selector a pick may carry. */
const MAX_PICK = 256;

/**
 * Tells whether the end of a pick can be trusted.
 *
 * @param {string|undefined} started Host recorded when the popup started the pick.
 * @param {{url?: string, incognito?: boolean}} tab The tab the message came from.
 * @param {{name?: unknown, first?: unknown, second?: unknown}} message What arrived.
 * @returns {string|null} The host, when the pick is good.
 */
export function pickedHost(started, tab, message) {
  const text = (value) => typeof value === "string" && value.length > 0 && value.length <= MAX_PICK;
  const second = message.second === null || message.second === undefined || text(message.second);
  if (!started || tab.incognito || pageHost(tab.url) !== started) return null;
  return text(message.name) && text(message.first) && second ? started : null;
}
