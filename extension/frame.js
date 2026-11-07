// Runs inside embedded frames, such as a video player from another site. It reads
// what a rule names for this frame and hands it to the service worker, which joins
// it to the page. A frame no rule asks for ends here, and costs nothing after that.
(() => {
  if (window.top === window) return;
  const running = globalThis.__rppFrame;
  if (running?.alive()) return;
  if (running) running.stop();

  const { extractValues, sameState } = globalThis.RPPExtract;
  const POLL_MS = 2000;
  const MEDIA_EVENTS = ["play", "pause", "seeked", "ratechange", "loadedmetadata"];

  let rules = [];
  const sent = new Map(); // presence -> { url, title, values, at }
  let timer = null;

  function alive() {
    return Boolean(chrome.runtime?.id);
  }

  function report() {
    if (!alive()) {
      stop();
      return;
    }
    for (const rule of rules) {
      const state = { url: "", title: "", values: extractValues(document, rule.extract), at: Date.now() };
      const previous = sent.get(rule.presence);
      if (previous && sameState(previous, state, state.at - previous.at)) continue;
      sent.set(rule.presence, state);
      chrome.runtime
        .sendMessage({ type: "frame", presence: rule.presence, values: state.values })
        .catch(() => sent.delete(rule.presence));
    }
  }

  function listen() {
    if (timer !== null) return;
    timer = setInterval(report, POLL_MS);
    for (const name of MEDIA_EVENTS) document.addEventListener(name, report, true);
    addEventListener("pagehide", leave);
  }

  function leave() {
    if (sent.size === 0) return;
    sent.clear();
    chrome.runtime.sendMessage({ type: "frame-left" }).catch(() => {});
  }

  function stop() {
    clearInterval(timer);
    timer = null;
    for (const name of MEDIA_EVENTS) document.removeEventListener(name, report, true);
    removeEventListener("pagehide", leave);
    if (globalThis.__rppFrame === handle) globalThis.__rppFrame = null;
  }

  async function start() {
    try {
      rules = (await chrome.runtime.sendMessage({ type: "frame-rules" })) || [];
    } catch {
      rules = [];
    }
    if (rules.length === 0) {
      stop();
      return;
    }
    listen();
    report();
  }

  const handle = { alive, stop };
  globalThis.__rppFrame = handle;
  chrome.runtime.onMessage.addListener((message) => {
    if (message && message.type === "rules-changed") start();
    // The worker slept and forgot what this frame said.
    if (message && message.type === "frames-resend") {
      sent.clear();
      report();
    }
  });
  start();
})();
