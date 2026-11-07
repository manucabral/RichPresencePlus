// Popup: the link to the app, the page it was opened on, the presences this
// browser reads, a pause switch, pairing by hand and site access.
import { EVERY_SITE, toOrigin } from "./pairing.js";
import { describeState, elapsed, presenceNames } from "./status.js";

const $ = (id) => document.getElementById(id);

// Texts come from _locales in the browser's language; the HTML keeps English
// for a key a translation lacks.
const t = (key, ...values) => chrome.i18n.getMessage(key, values.map(String)) || key;
document.documentElement.lang = chrome.i18n.getUILanguage();
for (const element of document.querySelectorAll("[data-i18n]")) {
  const text = chrome.i18n.getMessage(element.dataset.i18n);
  if (text) element.textContent = text;
}
const manifest = chrome.runtime.getManifest();
$("version").textContent = `v${manifest.version_name || manifest.version}`;

const BROWSERS = {
  chrome: "Google Chrome",
  edge: "Microsoft Edge",
  firefox: "Mozilla Firefox",
  opera: "Opera",
  brave: "Brave",
  vivaldi: "Vivaldi",
  yandex: "Yandex Browser",
};

// The tab the popup was opened on; `activeTab` lets it read the address.
async function activeTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab ? { id: tab.id, url: tab.url } : null;
}

function showStatus(state) {
  const { key, tone } = state.paused ? { key: "pausedNow", tone: "warn" } : describeState(state);
  $("state").textContent = t(key);
  $("dot").className = `dot ${tone}`;
  const parts = [];
  if (state.browser && state.browser.name !== "other") {
    parts.push(`${BROWSERS[state.browser.name] ?? state.browser.name} ${state.browser.version}`.trim());
  }
  if (state.connected && state.connectedAt) {
    const since = elapsed(state.connectedAt, Date.now());
    parts.push(t(since.key, since.count));
  }
  if (state.reading > 0) parts.push(t("reading", state.reading));
  $("detail").textContent = parts.join(", ");
  $("paused").checked = state.paused;
}

async function showPage(connected) {
  const tab = await activeTab();
  const info = tab ? await chrome.runtime.sendMessage({ type: "tab", tab }) : { presence: null };
  const allow = $("allowPage");
  const remove = $("removePage");
  const make = $("makeRow");
  allow.hidden = true;
  remove.hidden = true;
  // A presence that reads any site, like Web Media, leaves room for one of its own.
  make.hidden = !connected || !/^https?:/.test(tab?.url ?? "") || Boolean(info.presence && !info.open);
  $("makeSite").onclick = async () => {
    const answer = await chrome.runtime.sendMessage({ type: "pick-start", tab: tab.id });
    if (answer.ok) window.close();
    else $("page").textContent = t("pickFailed");
  };
  if (!tab?.url) {
    $("page").textContent = t("unknownPage");
    $("pageDot").className = "dot";
    return;
  }
  if (!info.presence) {
    $("page").textContent = t("noPresence");
    $("pageDot").className = "dot";
    return;
  }
  if (!info.granted) {
    $("page").textContent = t("needsAccess", info.presence);
    $("pageDot").className = "dot warn";
    allow.hidden = false;
    allow.onclick = async () => {
      // The popup can close while the browser asks, which would lose the choice, so
      // it is saved first. A site without the permission is never read. Waiting for
      // the message would take the request away from this click.
      const choice = (on) => chrome.runtime.sendMessage({ type: "open-site", origin: info.origins[0], on });
      if (info.open) choice(true);
      if (await chrome.permissions.request({ origins: info.origins })) {
        // A site another permission already covers changes nothing the worker can see.
        await chrome.runtime.sendMessage({ type: "refresh" });
        await refresh();
      } else if (info.open) {
        await choice(false);
      }
    };
    return;
  }
  if (info.open) {
    remove.hidden = false;
    remove.onclick = async () => {
      await chrome.runtime.sendMessage({ type: "open-site", origin: info.origins[0], on: false });
      if (!info.shared) await chrome.permissions.remove({ origins: info.origins });
      await chrome.runtime.sendMessage({ type: "refresh" });
      await refresh();
    };
  }
  $("page").textContent = info.reading ? t("readsPage", info.presence) : t("notReadYet", info.presence);
  $("pageDot").className = info.reading ? "dot on" : "dot";
}

async function showPresences(state) {
  const list = $("presences");
  list.replaceChildren();
  const names = presenceNames(state.rules);
  $("noRules").hidden = names.length > 0;
  const missing = new Set(state.missing);
  for (const name of names) {
    const origins = state.rules
      .filter((rule) => rule.presence === name)
      .flatMap((rule) => rule.matches.map(toOrigin))
      .filter(Boolean);
    const open = state.rules.some((rule) => rule.presence === name && rule.matches.includes(EVERY_SITE));
    const allowed = origins.every((origin) => !missing.has(origin));
    const item = document.createElement("li");
    item.className = "row";
    const dot = document.createElement("span");
    dot.className = allowed ? "dot on" : "dot warn";
    const label = document.createElement("span");
    label.className = "grow truncate";
    label.textContent = name;
    const tag = document.createElement("span");
    tag.className = allowed ? "tag on" : "tag";
    tag.textContent = open ? t("perSite") : allowed ? t("allowed") : t("notAllowed");
    item.append(dot, label, tag);
    list.append(item);
  }
}

async function refresh() {
  const state = await chrome.runtime.sendMessage({ type: "status" });
  showStatus(state);
  // The code is a fallback for when the extension cannot find the app.
  $("pair").hidden = state.connected;
  $("pair").open = !state.paired;
  $("allow").hidden = state.missing.length === 0;
  $("sites").textContent = state.missing.join(", ");
  await showPresences(state);
  await showPage(state.connected);
  return state;
}

$("paused").addEventListener("change", async (event) => {
  await chrome.runtime.sendMessage({ type: "pause", paused: event.target.checked });
  await refresh();
});

$("save").addEventListener("click", async () => {
  const answer = await chrome.runtime.sendMessage({ type: "pair", code: $("code").value });
  $("error").hidden = answer.ok;
  if (answer.ok) setTimeout(refresh, 800);
});

$("grant").addEventListener("click", async () => {
  const state = await chrome.runtime.sendMessage({ type: "status" });
  // The request must start from this click, so it is made here.
  if (await chrome.permissions.request({ origins: state.missing })) await refresh();
});

refresh();
