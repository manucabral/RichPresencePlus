// Runs in the page's own world from the start of the load, only on pages of a rule that
// reads answers. It wraps `fetch` and `XMLHttpRequest` to look at the JSON answers of the
// addresses a rule names, and keeps only the value at each path. Requests are left as the
// page made them and nothing leaves the page: the page script asks for the values, and
// checks what it receives.
(() => {
  const MARK = Symbol.for("rpp.responses");
  if (window[MARK]) return;
  Object.defineProperty(window, MARK, { value: true });

  // Longest answer read; a longer one is left alone, and its reading stops there.
  const MAX_BODY = 1024 * 1024;
  // Longest value kept, as JSON.
  const MAX_VALUE = 4096;
  // Answers that arrive before the page script names its addresses wait here, within
  // these limits, and only for the first seconds of the page.
  const MAX_EARLY = 32;
  const MAX_EARLY_BYTES = 2 * 1024 * 1024;
  const EARLY_MS = 30000;

  // presence -> [{ name, test, path }], as the page script last asked.
  const watched = new Map();
  const asked = new Map(); // presence -> the request text, to skip rebuilding the same
  // presence -> { name: value }
  const kept = new Map();
  let early = [];
  let earlyBytes = 0;
  setTimeout(() => {
    early = null;
  }, EARLY_MS);

  // A Chrome match pattern as a test of an address, built once per pattern. One that
  // starts with `/` is a path on the page's own site, for servers people run themselves.
  function tester(pattern) {
    const own = pattern.startsWith("/");
    const split = own ? [pattern, "", "", pattern] : /^(\*|https?):\/\/([^/]*)(\/.*)$/.exec(pattern);
    if (!split) return () => false;
    const [, scheme, host, path] = split;
    const route = new RegExp(`^${path.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*")}$`);
    return (address) => {
      let url;
      try {
        url = new URL(address, location.href);
      } catch {
        return false;
      }
      if (url.protocol !== "http:" && url.protocol !== "https:") return false;
      if (own) return url.origin === location.origin && route.test(url.pathname + url.search);
      if (scheme !== "*" && `${scheme}:` !== url.protocol) return false;
      if (host.startsWith("*.")) {
        const base = host.slice(2);
        if (url.hostname !== base && !url.hostname.endsWith(`.${base}`)) return false;
      } else if (url.hostname !== host) {
        return false;
      }
      return route.test(url.pathname + url.search);
    };
  }

  function walk(value, path) {
    let found = value;
    for (const part of path ? path.split(".") : []) {
      if (found === null || typeof found !== "object" || !Object.hasOwn(found, part)) return null;
      found = found[part];
    }
    return found;
  }

  function small(value) {
    try {
      const text = JSON.stringify(value);
      return text !== undefined && text.length <= MAX_VALUE ? JSON.parse(text) : null;
    } catch {
      return null;
    }
  }

  function wanted(address) {
    for (const list of watched.values()) {
      if (list.some((entry) => entry.test(address))) return true;
    }
    return false;
  }

  // Whether an answer is worth reading: one a rule watches, or, before any rule spoke,
  // a JSON answer while there is room to keep it.
  function worth(address, type) {
    if (!address) return false;
    if (wanted(address)) return true;
    return early !== null && early.length < MAX_EARLY && /json/i.test(type || "");
  }

  // Keeps, for every entry that watches the address, the value at its path.
  function take(address, text, data) {
    if (!wanted(address)) {
      if (early !== null && typeof text === "string" && early.length < MAX_EARLY) {
        if (earlyBytes + text.length <= MAX_EARLY_BYTES) {
          early.push({ address, text });
          earlyBytes += text.length;
        }
      }
      return;
    }
    let body = data;
    let read = data !== undefined;
    for (const [presence, list] of watched) {
      for (const entry of list) {
        if (!entry.test(address)) continue;
        if (!read) {
          read = true;
          try {
            body = JSON.parse(text);
          } catch {
            return; // Not JSON after all.
          }
        }
        kept.get(presence)[entry.name] = small(walk(body, entry.path));
      }
    }
  }

  // Reads a stream of text up to MAX_BODY, and gives up past it.
  async function readCapped(stream) {
    const reader = stream.getReader();
    const decoder = new TextDecoder();
    let text = "";
    let size = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) return text + decoder.decode();
      size += value.byteLength;
      if (size > MAX_BODY) {
        reader.cancel().catch(() => {});
        return null;
      }
      text += decoder.decode(value, { stream: true });
    }
  }

  function look(response) {
    try {
      if (!response || response.type === "opaque" || !response.body) return;
      const type = response.headers.get("content-type");
      if (!worth(response.url, type)) return;
      const length = Number(response.headers.get("content-length"));
      if (length > MAX_BODY) return;
      const address = response.url;
      readCapped(response.clone().body).then(
        (text) => text !== null && take(address, text),
        () => {},
      );
    } catch {
      // A response the page already consumed, or a body that cannot be cloned.
    }
  }

  const realFetch = window.fetch;
  if (typeof realFetch === "function") {
    // Named and shaped as the original, so the page sees the same function.
    window.fetch = function fetch(_input) {
      // biome-ignore lint/complexity/noArguments: every argument passes as the page gave it.
      const pending = Reflect.apply(realFetch, this, arguments);
      pending.then(look, () => {});
      return pending;
    };
  }

  const Request = window.XMLHttpRequest;
  if (typeof Request === "function") {
    const proto = Request.prototype;
    const realOpen = proto.open;
    const listening = new WeakSet();
    const onLoad = function () {
      try {
        const address = this.responseURL;
        if (!worth(address, this.getResponseHeader("content-type"))) return;
        if (this.responseType === "json") {
          if (this.response !== null) take(address, undefined, this.response);
        } else if (this.responseType === "" || this.responseType === "text") {
          const text = this.responseText;
          if (text.length <= MAX_BODY) take(address, text);
        }
      } catch {
        // An answer the page aborted.
      }
    };
    proto.open = function open(_method, _url) {
      if (!listening.has(this)) {
        listening.add(this);
        this.addEventListener("load", onLoad);
      }
      // biome-ignore lint/complexity/noArguments: every argument passes as the page gave it.
      return Reflect.apply(realOpen, this, arguments);
    };
  }

  // The page script names what a presence reads, and receives the values kept for it.
  document.addEventListener("rpp-responses", (event) => {
    let request;
    try {
      request = JSON.parse(event.detail);
    } catch {
      return;
    }
    const { presence, entries } = request || {};
    if (typeof presence !== "string" || !Array.isArray(entries)) return;
    if (asked.get(presence) !== event.detail) {
      asked.set(presence, event.detail);
      watched.set(
        presence,
        entries.map((entry) => ({ name: String(entry.name), test: tester(String(entry.url)), path: entry.path })),
      );
      const before = kept.get(presence) || {};
      const values = {};
      for (const entry of entries) values[entry.name] = before[entry.name] ?? null;
      kept.set(presence, values);
      // What arrived before anyone asked is read once. The waiting ends after this
      // report, which asks for every presence of the page in the same task.
      if (early !== null) {
        const waiting = early;
        early = [];
        earlyBytes = 0;
        for (const { address, text } of waiting) take(address, text);
        queueMicrotask(() => {
          early = null;
          earlyBytes = 0;
        });
      }
    }
    document.dispatchEvent(new CustomEvent("rpp-answers", { detail: JSON.stringify(kept.get(presence)) }));
  });
})();
