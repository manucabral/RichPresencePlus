// Which browser the extension runs in, told to the desktop app in the hello.
//
// Browsers built on Chromium copy Chrome's user agent and add a token of
// their own (`YaBrowser/`, `OPR/`, `Edg/`), and recent ones also list
// themselves in `navigator.userAgentData.brands`. Brave hides from both and
// is found through `navigator.brave`. Tokens are checked before `Chrome/`,
// since every one of these user agents contains it.

// Name sent to the app, the user agent token and the brand of each browser.
const KNOWN = [
  { name: "firefox", token: "Firefox", brand: null },
  { name: "yandex", token: "YaBrowser", brand: "Yandex" },
  { name: "edge", token: "Edg", brand: "Microsoft Edge" },
  { name: "opera", token: "OPR", brand: "Opera" },
  { name: "vivaldi", token: "Vivaldi", brand: "Vivaldi" },
  { name: "brave", token: null, brand: "Brave" },
  { name: "chrome", token: "Chrome", brand: "Google Chrome" },
];

// Digits and dots only, as the app accepts.
const VERSION = /^[0-9.]{1,24}$/;

function tokenVersion(userAgent, token) {
  if (!token) return null;
  const found = new RegExp(`(?:^|[\\s(])${token}/([0-9.]+)`).exec(userAgent);
  return found ? found[1] : null;
}

/**
 * Names the browser and its version.
 *
 * @param {object} facts What the page can read about itself.
 * @param {string} facts.userAgent `navigator.userAgent`.
 * @param {{brand: string, version: string}[]} [facts.brands] `navigator.userAgentData.brands`.
 * @param {boolean} [facts.brave] Whether `navigator.brave` exists.
 * @returns {{name: string, version: string}} A name the app knows, or "other",
 *     and the version, or "" when it cannot be read.
 */
export function detectBrowser({ userAgent = "", brands = [], brave = false }) {
  const listed = new Map(brands.map((entry) => [entry.brand, entry.version]));
  const known =
    (brave && KNOWN.find((entry) => entry.name === "brave")) ||
    KNOWN.find((entry) => entry.brand && entry.name !== "chrome" && listed.has(entry.brand)) ||
    KNOWN.find((entry) => tokenVersion(userAgent, entry.token) !== null) ||
    null;
  if (!known) return { name: "other", version: "" };
  // The user agent has the full version; a brand only has the major one.
  const version =
    tokenVersion(userAgent, known.token) ??
    listed.get(known.brand) ??
    (known.name === "brave" ? tokenVersion(userAgent, "Chrome") : null) ??
    "";
  return { name: known.name, version: VERSION.test(version) ? version : "" };
}

/** The facts `detectBrowser` needs, read from this context. */
export function currentFacts() {
  return {
    userAgent: navigator.userAgent,
    brands: navigator.userAgentData?.brands ?? [],
    brave: typeof navigator.brave !== "undefined",
  };
}
