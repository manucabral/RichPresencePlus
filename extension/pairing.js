// Small pure helpers shared by the service worker and the popup.

const CODE = /^(\d{1,5}):([0-9a-f]{64})$/i;
const TOKEN = /^[0-9a-f]{64}$/;

// Native messaging host of the desktop app (rpp-bridge/src/native.rs).
export const HOST = "com.manucabral.richpresenceplus";
// While the app does not answer, the host is asked at most this often.
export const HOST_RETRY_MS = 60000;

// Reads the pairing code shown by the desktop application ("port:token").
export function parsePairingCode(text) {
  const found = CODE.exec(String(text).trim());
  if (!found) return null;
  const port = Number(found[1]);
  if (port < 1 || port > 65535) return null;
  return { port, token: found[2].toLowerCase() };
}

// Reads the native host's answer. Returns the pairing, or null when the app
// has not recorded one or the answer is malformed.
export function parseHostAnswer(answer) {
  if (answer?.type !== "pairing") return null;
  const { port, token } = answer;
  if (!Number.isInteger(port) || port < 1 || port > 65535) return null;
  if (typeof token !== "string" || !TOKEN.test(token)) return null;
  return { port, token, auto: true };
}

// Whether to ask the host before connecting. Without a pairing it is always
// asked; with one, again after a while, since the app may have changed port.
export function shouldAskHost(pairing, lastAsked, now) {
  return !pairing || now - lastAsked >= HOST_RETRY_MS;
}

// The pairing to use after asking the host: the app's answer when it gave
// one, otherwise what was stored.
export function choosePairing(stored, answer) {
  const fresh = parseHostAnswer(answer);
  return fresh || stored || null;
}

// Two pairings lead to the same connection.
export function samePairing(a, b) {
  return Boolean(a && b && a.port === b.port && a.token === b.token);
}

// Wait before reconnecting after `attempt` failures, in milliseconds.
export function nextDelay(attempt) {
  return Math.min(30000, 1000 * 2 ** Math.min(attempt, 5));
}

// A presence for every site runs only on the sites the user allows one by one.
export const EVERY_SITE = "*://*/*";

export function isOpenRule(rule) {
  return rule.matches.includes(EVERY_SITE);
}

// Host permission pattern for the page at an address.
export function pageOrigin(address) {
  try {
    const url = new URL(address);
    return /^https?:$/.test(url.protocol) ? `${url.protocol}//${url.hostname}/*` : null;
  } catch {
    return null;
  }
}

// Host permission pattern for a match pattern: the path is always /*.
export function toOrigin(pattern) {
  const found = /^(\*|https?):\/\/([^/]*)\//.exec(pattern);
  return found ? `${found[1]}://${found[2]}/*` : null;
}

// Unique origins needed by a list of rules.
export function originsOf(rules) {
  const origins = new Set();
  for (const rule of rules) {
    for (const pattern of [...rule.matches, ...(rule.frames?.matches ?? [])]) {
      const origin = toOrigin(pattern);
      if (origin) origins.add(origin);
    }
  }
  return [...origins];
}

// The sites a presence for every site may read: the ones the user chose for it
// and the browser still allows. A permission given to another presence does not
// count, so turning that presence off hands nothing over.
export function openSites(chosen, granted) {
  return chosen.filter((origin) => origin !== EVERY_SITE && granted.includes(origin));
}

// The rules a page may use: a rule for every site only where it was chosen.
export function rulesForOrigin(rules, origin, sites) {
  return rules.filter((rule) => !isOpenRule(rule) || sites.includes(origin));
}

// Whether a rule made for some sites reads this host, so it needs its permission.
export function usesHost(rules, address) {
  let host;
  try {
    host = new URL(address).hostname;
  } catch {
    return false;
  }
  return rules.some((rule) => !isOpenRule(rule) && originsOf([rule]).some((origin) => origin.endsWith(`://${host}/*`)));
}
