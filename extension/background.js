// Service worker: keeps the connection to the desktop application, registers
// the content scripts the rules need and forwards what they read.

import { currentFacts, detectBrowser } from "./browser.js";
import { fitsLink, mayReceive, pageHost, pickedHost } from "./messages.js";
import {
  choosePairing,
  HOST,
  isOpenRule,
  nextDelay,
  openSites,
  originsOf,
  pageOrigin,
  parsePairingCode,
  rulesForOrigin,
  samePairing,
  shouldAskHost,
  usesHost,
} from "./pairing.js";
import { checkProof, createSession, newNonce, proof } from "./secure.js";
import { ruleForUrl } from "./status.js";
// A classic script shared with the content scripts; it sets `RPPExtract`.
import "./extract.js";

const PROTOCOL = 2;
const CONTENT_FILES = ["extract.js", "content.js"];
const FRAME_FILES = ["extract.js", "frame.js"];
const VARIABLE_FILES = ["variables.js"];
const RESPONSE_FILES = ["responses.js"];
const HEARTBEAT_MS = 20000;

let socket = null;
let connecting = false;
let connected = false;
// The sealed session of the open connection; null until the app proved itself.
let session = null;
// Sealing is asynchronous, so messages queue here to leave in their numbered order.
let outbox = Promise.resolve();
// Set when a program on the port could not prove it is the desktop app.
let untrusted = false;
let attempt = 0;
let heartbeat = null;
let rules = [];
// When the native host was last asked, and what came of it: "unknown",
// "found", "absent" (the app never ran) or "missing" (no host registered).
let hostAsked = 0;
let host = "unknown";
// When the link to the app opened, for the popup; null while disconnected.
let connectedAt = null;
// Tabs whose page was sent, by tab id, so a pause can take them back.
const reported = new Map();
// The last page of each tab, so a frame that changes can send it again.
const lastPages = new Map();
// What the frames of a tab said: `${tab}/${presence}` -> frame id -> { values, at }.
const frameValues = new Map();
// The user paused this browser: pages are read but nothing is sent.
let paused = false;
const pausedLoaded = chrome.storage.local.get("paused").then((stored) => {
  paused = Boolean(stored.paused);
});

// Rules survive a worker restart, so content scripts can ask for them at once.
const ready = chrome.storage.session.get("rules").then((stored) => {
  rules = stored.rules || [];
});

// Read once: the browser does not change while the worker lives.
const BROWSER = detectBrowser(currentFacts());

function send(message) {
  const ws = socket;
  const sealer = session;
  if (!ws || !sealer || !connected) return;
  outbox = outbox
    .then(async () => {
      const sealed = await sealer.seal(JSON.stringify(message));
      if (ws.readyState === WebSocket.OPEN) ws.send(sealed);
    })
    .catch(() => ws.close());
}

// A pick survives the worker going to sleep, which it does while someone reads
// the page. Session storage is out of reach of the scripts in pages.
const pickKey = (tab) => `pick-${tab}`;

async function startPick(tabId) {
  const tab = await chrome.tabs.get(tabId);
  const host = pageHost(tab.url);
  if (!host || tab.incognito || !connected) return { ok: false };
  await chrome.storage.session.set({ [pickKey(tabId)]: host });
  await chrome.scripting.executeScript({ target: { tabId }, files: ["selector.js", "picker.js"] });
  return { ok: true };
}

async function finishPick(tab, message) {
  const key = pickKey(tab.id);
  const started = (await chrome.storage.session.get(key))[key];
  await chrome.storage.session.remove(key);
  const host = pickedHost(started, tab, message);
  if (!host || !connected) return { ok: false };
  send({ type: "make_site", name: message.name, host, first: message.first, second: message.second ?? null });
  return { ok: true };
}

const { declaredValues, framesForPage, joinFrames, responseEntries, rulesForPage, variablePaths } =
  globalThis.RPPExtract;

const readsVariables = (rule) => variablePaths(rule.extract).length > 0;
const readsResponses = (rule) => responseEntries(rule.extract).length > 0;

// Sends a page with what its frames said. A page can make its address as long as
// it likes; the app closes the link at a message that is too long, and every tab
// would lose it.
function publish(tab, page) {
  const frames = frameValues.get(`${tab}/${page.presence}`);
  const joined = frames ? joinFrames([...frames.values()], Date.now()) : {};
  const full = { ...page, values: { ...page.values, ...joined } };
  if (fitsLink(full)) {
    reported.set(tab, page.presence);
    send(full);
  } else if (reported.delete(tab)) {
    send({ type: "gone", tab });
  }
}

// A frame said something new: keep it, and send its page again.
async function frameSaid(tab, sender, message) {
  await ready;
  const rule = framesForPage(rules, sender.tab.url, sender.url).find((item) => item.presence === message.presence);
  if (!rule) return;
  const key = `${tab}/${message.presence}`;
  const frames = frameValues.get(key) ?? new Map();
  frames.set(sender.frameId, { values: declaredValues(rule.extract, message.values), at: Date.now() });
  frameValues.set(key, frames);
  const page = lastPages.get(tab);
  if (page && page.presence === message.presence) publish(tab, page);
  // The worker slept and lost the page; it is still open.
  else chrome.tabs.sendMessage(tab, { type: "resend" }, { frameId: 0 }).catch(() => {});
}

function frameLeft(tab, frameId) {
  for (const [key, frames] of frameValues) {
    if (!key.startsWith(`${tab}/`) || !frames.delete(frameId)) continue;
    const page = lastPages.get(tab);
    if (page && !paused) publish(tab, page);
  }
}

function forgetTab(tab) {
  lastPages.delete(tab);
  for (const key of frameValues.keys()) {
    if (key.startsWith(`${tab}/`)) frameValues.delete(key);
  }
}

// The sites the user chose for a presence that can run on any of them.
async function allowedSites() {
  const { openSites: chosen = [] } = await chrome.storage.local.get("openSites");
  // The browser answers for each site: a wider permission covers one without
  // listing it, and comparing the lists would ask again for ever.
  const allowed = await Promise.all(
    chosen.map((origin) => chrome.permissions.contains({ origins: [origin] }).catch(() => false)),
  );
  return openSites(
    chosen,
    chosen.filter((_, index) => allowed[index]),
  );
}

// Adds a site to those chosen for a presence for every site, or takes it out.
async function chooseSite(origin, on) {
  const { openSites: chosen = [] } = await chrome.storage.local.get("openSites");
  const others = chosen.filter((item) => item !== origin);
  await chrome.storage.local.set({ openSites: on ? [...others, origin] : others });
}

async function granted(rule) {
  if (isOpenRule(rule)) return (await allowedSites()).length > 0;
  const origins = originsOf([rule]);
  return origins.length > 0 && (await chrome.permissions.contains({ origins }));
}

// Rule updates can arrive close together (a reconnect, a refresh from the
// popup). They are applied one after another: two at once would both try to
// register the same script ids and the second would fail halfway.
let applying = Promise.resolve();

function applyRules(next) {
  // A failed update must not block the ones after it.
  applying = applying.catch(() => {}).then(() => registerRules(next));
  return applying;
}

// Registers one content script per rule the user allowed on its sites.
async function registerRules(next) {
  rules = next;
  await chrome.storage.session.set({ rules });
  const allowed = [];
  for (const rule of rules) {
    if (await granted(rule)) allowed.push(rule);
  }
  const registered = await chrome.scripting.getRegisteredContentScripts();
  const stale = registered.filter((s) => s.id.startsWith("rpp-")).map((s) => s.id);
  if (stale.length > 0) await chrome.scripting.unregisterContentScripts({ ids: stale });
  const sites = await allowedSites();
  const matchesOf = (rule) => (isOpenRule(rule) ? sites : rule.matches);
  if (allowed.length > 0) {
    const script = (id, matches, js, more = {}) => ({
      id,
      matches,
      js,
      runAt: "document_idle",
      persistAcrossSessions: true,
      ...more,
    });
    await chrome.scripting.registerContentScripts(
      allowed.flatMap((rule, index) => [
        script(`rpp-${index}`, matchesOf(rule), CONTENT_FILES),
        // Only what a rule needs runs: most pages have no frames or variables to read.
        ...(rule.frames ? [script(`rpp-frame-${index}`, rule.frames.matches, FRAME_FILES, { allFrames: true })] : []),
        ...(readsVariables(rule)
          ? [script(`rpp-variables-${index}`, matchesOf(rule), VARIABLE_FILES, { world: "MAIN" })]
          : []),
        // Answers start before the page is idle, so this one runs from the first moment.
        ...(readsResponses(rule)
          ? [
              script(`rpp-responses-${index}`, matchesOf(rule), RESPONSE_FILES, {
                world: "MAIN",
                runAt: "document_start",
              }),
            ]
          : []),
      ]),
    );
  }
  // Tabs that were already open do not receive registered scripts. The
  // script guards against running twice in the same page.
  const matches = [...new Set(allowed.flatMap(matchesOf))];
  if (matches.length > 0) {
    for (const tab of await chrome.tabs.query({ url: matches })) {
      const page = allowed.filter((rule) => matchesOf(rule).length > 0 && rulesForPage([rule], tab.url).length > 0);
      if (page.some(readsVariables)) {
        chrome.scripting
          .executeScript({ target: { tabId: tab.id }, files: VARIABLE_FILES, world: "MAIN" })
          .catch(() => {});
      }
      // An open tab only shows answers that arrive from now on.
      if (page.some(readsResponses)) {
        chrome.scripting
          .executeScript({ target: { tabId: tab.id }, files: RESPONSE_FILES, world: "MAIN" })
          .catch(() => {});
      }
      chrome.scripting.executeScript({ target: { tabId: tab.id }, files: CONTENT_FILES }).catch(() => {});
      if (page.some((rule) => rule.frames)) {
        chrome.scripting
          .executeScript({ target: { tabId: tab.id, allFrames: true }, files: FRAME_FILES })
          .catch(() => {});
      }
    }
  }
  // Scripts already running ask for the new rules; tabs without one ignore it.
  for (const tab of await chrome.tabs.query({})) {
    chrome.tabs.sendMessage(tab.id, { type: "rules-changed" }).catch(() => {});
  }
}

// Firefox gives promises on `browser`; Chrome and Edge on `chrome`.
const runtime = globalThis.browser?.runtime ?? chrome.runtime;

// Asks the desktop app's native host for the port and token.
async function askHost() {
  hostAsked = Date.now();
  try {
    const answer = await runtime.sendNativeMessage(HOST, { type: "pairing" });
    host = answer && answer.type === "pairing" ? "found" : "absent";
    return answer;
  } catch {
    // The browser could not start the host: the app is not installed, or has
    // not run once to register it.
    host = "missing";
    return null;
  }
}

// The stored pairing, refreshed from the native host when it is time.
async function currentPairing() {
  const { pairing: stored } = await chrome.storage.local.get("pairing");
  if (!shouldAskHost(stored, hostAsked, Date.now())) return stored;
  const pairing = choosePairing(stored, await askHost());
  if (pairing && !samePairing(pairing, stored)) await chrome.storage.local.set({ pairing });
  return pairing;
}

// Forgets the open connection. The app drops a closed connection's pages, so
// nothing is left to take back. Without this, a socket that replaces another
// would seal messages with the old session until its own handshake ends.
function dropLink() {
  socket = null;
  session = null;
  connected = false;
  connectedAt = null;
  reported.clear();
  clearInterval(heartbeat);
}

function schedule() {
  attempt += 1;
  setTimeout(connect, nextDelay(attempt));
}

async function connect() {
  // Startup events and the top-level call can arrive together; the flag keeps
  // the awaited storage read from letting two of them open a socket each.
  if (connecting || (socket && socket.readyState <= WebSocket.OPEN)) return;
  connecting = true;
  let pairing;
  try {
    pairing = await currentPairing();
  } finally {
    connecting = false;
  }
  if (!pairing || (socket && socket.readyState <= WebSocket.OPEN)) return;
  const ws = new WebSocket(`ws://127.0.0.1:${pairing.port}/`);
  socket = ws;
  // Each message is handled after the previous one: the handshake has steps,
  // and sealed messages must be opened in order.
  let inbox = Promise.resolve();
  const link = { server: null, client: null, session: null };
  ws.onmessage = (event) => {
    inbox = inbox.then(() => receive(ws, pairing.token, link, event.data)).catch(() => ws.close());
  };
  ws.onclose = () => {
    if (socket !== ws) return;
    dropLink();
    // The app answered but refused the proof: the stored token is old, so the
    // next try asks the host instead of repeating it for a minute.
    if (link.server && !link.session) hostAsked = 0;
    schedule();
  };
  ws.onerror = () => {};
}

// One step of the conversation: challenge, then welcome, then sealed messages.
async function receive(ws, token, link, text) {
  const message = JSON.parse(text);
  if (!link.server) {
    if (message.type !== "challenge" || message.version !== PROTOCOL) throw new Error("unexpected first message");
    link.server = message.nonce;
    link.client = newNonce();
    const answer = await proof(token, "rpp-hello", link.server, link.client);
    ws.send(
      JSON.stringify({
        type: "hello",
        browser: BROWSER.name,
        browser_version: BROWSER.version,
        extension_version: chrome.runtime.getManifest().version,
        version: PROTOCOL,
        nonce: link.client,
        proof: answer,
      }),
    );
    return;
  }
  if (!link.session) {
    const genuine =
      message.type === "welcome" && (await checkProof(token, "rpp-welcome", link.server, link.client, message.proof));
    if (!genuine) {
      untrusted = true;
      throw new Error("the program on this port is not the desktop app");
    }
    link.session = await createSession(token, link.server, link.client);
    session = link.session;
    untrusted = false;
    connected = true;
    connectedAt = Date.now();
    attempt = 0;
    clearInterval(heartbeat);
    heartbeat = setInterval(() => send({ type: "ping" }), HEARTBEAT_MS);
    // A new connection starts without pages; the open tabs send theirs again.
    askForPages().catch(() => {});
    return;
  }
  const inner = JSON.parse(await link.session.open(message));
  if (inner.type === "rules") {
    applyRules(inner.rules || []).catch((error) => console.warn("rules failed", error));
  }
}

async function status() {
  const { pairing } = await chrome.storage.local.get("pairing");
  const missing = [];
  for (const rule of rules) {
    // Asking for every site at once is what the per-site permission avoids.
    if (!isOpenRule(rule) && !(await granted(rule))) missing.push(...originsOf([rule]));
  }
  await pausedLoaded;
  return {
    paired: Boolean(pairing),
    auto: Boolean(pairing?.auto),
    connected,
    untrusted,
    host,
    missing: [...new Set(missing)],
    browser: BROWSER,
    connectedAt,
    paused,
    rules: rules.map((rule) => ({ presence: rule.presence, matches: rule.matches })),
    reading: reported.size,
  };
}

// What the popup shows for the page it was opened on.
async function tabInfo(tab) {
  await ready;
  const rule = ruleForUrl(rules, tab?.url, globalThis.RPPExtract.matchesPattern);
  if (!rule) return { presence: null };
  const open = isOpenRule(rule);
  const origins = open ? [pageOrigin(tab.url)].filter(Boolean) : originsOf([rule]);
  return {
    presence: rule.presence,
    open,
    granted: open ? origins.length > 0 && (await allowedSites()).includes(origins[0]) : await granted(rule),
    // Another presence reads this host, so its permission stays.
    shared: usesHost(rules, tab.url),
    reading: reported.has(tab.id),
    origins,
  };
}

// Asks every page script to send its page again, for an app that forgot them.
async function askForPages() {
  for (const tab of await chrome.tabs.query({})) {
    chrome.tabs.sendMessage(tab.id, { type: "resend" }).catch(() => {});
  }
}

// Pauses or resumes this browser; pausing takes back every page already sent.
async function setPaused(value) {
  paused = Boolean(value);
  await chrome.storage.local.set({ paused });
  if (paused) {
    for (const tab of reported.keys()) send({ type: "gone", tab });
    reported.clear();
  } else {
    await askForPages();
  }
}

const SELF = { id: chrome.runtime.id, base: chrome.runtime.getURL("") };

chrome.runtime.onMessage.addListener((message, sender, respond) => {
  if (!mayReceive(message, sender, SELF)) return false;
  // Private windows are never reported, even when the user lets the
  // extension run in them.
  const tab = sender.tab && !sender.tab.incognito ? sender.tab.id : null;
  switch (message.type) {
    case "rules":
      // A page that is never reported needs no rules: its script stays idle.
      ready
        .then(() => allowedSites())
        .then((sites) => respond(tab === null ? [] : rulesForOrigin(rules, pageOrigin(sender.tab.url), sites)));
      return true;
    case "page":
      // While paused, nothing leaves this browser.
      if (tab !== null && !paused) {
        const page = {
          type: "page",
          tab,
          presence: message.presence,
          url: sender.tab.url || message.url,
          title: sender.tab.title || message.title || "",
          values: message.values,
          visible: message.visible === true,
          focused: message.focused === true,
        };
        lastPages.set(tab, page);
        publish(tab, page);
        // After a nap the worker no longer knows what the frames said.
        if (!frameValues.has(`${tab}/${message.presence}`)) {
          chrome.tabs.sendMessage(tab, { type: "frames-resend" }).catch(() => {});
        }
      }
      return false;
    case "left":
      if (tab !== null) {
        reported.delete(tab);
        forgetTab(tab);
        send({ type: "gone", tab });
      }
      return false;
    case "frame-rules":
      ready.then(() => respond(tab === null ? [] : framesForPage(rules, sender.tab.url, sender.url)));
      return true;
    case "frame":
      if (tab !== null && !paused) frameSaid(tab, sender, message);
      return false;
    case "frame-left":
      if (tab !== null) frameLeft(tab, sender.frameId);
      return false;
    case "tab":
      tabInfo(message.tab).then(respond);
      return true;
    case "open-site":
      chooseSite(message.origin, message.on).then(() => respond({ ok: true }));
      return true;
    case "pause":
      setPaused(message.paused).then(() => respond({ paused }));
      return true;
    case "status":
      status().then(respond);
      return true;
    case "pair": {
      const pairing = parsePairingCode(message.code);
      if (!pairing) {
        respond({ ok: false });
        return false;
      }
      chrome.storage.local.set({ pairing }).then(() => {
        const old = socket;
        dropLink();
        if (old) old.close();
        attempt = 0;
        // A code typed by hand is tried before the host is asked again.
        hostAsked = Date.now();
        connect();
        respond({ ok: true });
      });
      return true;
    }
    case "pick-start":
      startPick(message.tab).then(respond, () => respond({ ok: false }));
      return true;
    case "pick-done":
      finishPick(sender.tab, message).then(respond, () => respond({ ok: false }));
      return true;
    case "pick-cancel":
      chrome.storage.session.remove(pickKey(sender.tab.id)).then(() => respond({ ok: true }));
      return true;
    case "refresh":
      applyRules(rules).then(
        () => respond({ ok: true }),
        () => respond({ ok: false }),
      );
      return true;
    default:
      return false;
  }
});

// The popup can close while the browser asks, so the worker follows the answer itself.
const followPermissions = () =>
  ready.then(() => applyRules(rules)).catch((error) => console.warn("rules failed", error));
chrome.permissions.onAdded.addListener(followPermissions);
chrome.permissions.onRemoved.addListener(followPermissions);

chrome.tabs.onRemoved.addListener((tab) => {
  forgetTab(tab);
  if (reported.delete(tab)) send({ type: "gone", tab });
});
// The worker sleeps while the app is closed, so its own timers do not run.
// Chrome allows 30 seconds; Firefox stretches this to one minute.
chrome.alarms.create("rpp-reconnect", { periodInMinutes: 0.5 });
chrome.alarms.onAlarm.addListener(() => connect());
chrome.runtime.onStartup.addListener(() => connect());
chrome.runtime.onInstalled.addListener(() => connect());
connect();
