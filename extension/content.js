// Runs on pages that match a rule. It reads the values the rule names and
// sends them to the service worker whenever they change.
(() => {
  // An update leaves the old script running in tabs that were already open,
  // cut off from the worker. It gives way to the new one.
  const running = globalThis.__rppContent;
  if (running?.alive()) return;
  if (running) running.stop();

  const { extractValues, sameState, rulesForPage, variablePaths } = globalThis.RPPExtract;
  const POLL_MS = 2000;

  let rules = [];
  const sent = new Map(); // presence -> { url, title, values, at }
  const MEDIA_EVENTS = ["play", "pause", "seeked", "ratechange", "loadedmetadata"];
  let timer = null;

  function alive() {
    return Boolean(chrome.runtime?.id);
  }

  async function loadRules() {
    try {
      rules = (await chrome.runtime.sendMessage({ type: "rules" })) || [];
    } catch {
      // The service worker is starting; the next poll asks again.
    }
  }

  // Asks the page's own world for its variables. The script there answers within
  // the call, so a rule with none pays nothing.
  function readVariables(extract) {
    const paths = variablePaths(extract);
    if (paths.length === 0) return null;
    let answer = null;
    const take = (event) => {
      try {
        answer = JSON.parse(event.detail);
      } catch {
        answer = null;
      }
    };
    document.addEventListener("rpp-values", take);
    document.dispatchEvent(new CustomEvent("rpp-read", { detail: JSON.stringify(paths) }));
    document.removeEventListener("rpp-values", take);
    return answer;
  }

  function report() {
    // Once cut off, every call to the worker throws; the timer ends with it.
    if (!alive()) {
      stop();
      return;
    }
    const address = location.href;
    for (const rule of rulesForPage(rules, address)) {
      const state = {
        url: address,
        title: document.title,
        values: extractValues(document, rule.extract, readVariables(rule.extract)),
        at: Date.now(),
      };
      const previous = sent.get(rule.presence);
      if (previous && sameState(previous, state, state.at - previous.at)) continue;
      sent.set(rule.presence, state);
      chrome.runtime
        .sendMessage({
          type: "page",
          presence: rule.presence,
          url: state.url,
          title: state.title,
          values: state.values,
        })
        .catch(() => sent.delete(rule.presence));
    }
  }

  function leave() {
    if (sent.size === 0) return;
    sent.clear();
    chrome.runtime.sendMessage({ type: "left" }).catch(() => {});
  }

  function stop() {
    clearInterval(timer);
    for (const name of MEDIA_EVENTS) document.removeEventListener(name, report, true);
    removeEventListener("pagehide", leave);
    if (globalThis.__rppContent === handle) globalThis.__rppContent = null;
  }

  const handle = { alive, stop };
  globalThis.__rppContent = handle;

  loadRules().then(report);
  timer = setInterval(report, POLL_MS);
  // The service worker announces new rules, so they are not polled: polling
  // would wake the worker from every open tab.
  chrome.runtime.onMessage.addListener((message) => {
    if (message && message.type === "rules-changed") loadRules().then(report);
    // The app forgot this page (a new connection, or the user resumed):
    // send it again even though nothing changed.
    if (message && message.type === "resend") {
      sent.clear();
      report();
    }
  });
  // Media events do not bubble, so listen while capturing.
  for (const name of MEDIA_EVENTS) {
    document.addEventListener(name, report, true);
  }
  addEventListener("pagehide", leave);
})();
