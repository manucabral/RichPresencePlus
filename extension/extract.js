// Reads the values a rule asks for. This file runs inside web pages (as a
// content script) and in Node for the tests, so it only uses standard DOM
// calls and exports through `globalThis.RPPExtract` or `module.exports`.
((root) => {
  const MAX_TEXT = 512;
  // The app keeps no longer address; a page can write one of any size.
  const MAX_ADDRESS = 2048;
  // Items of a list, unless the manifest asks for fewer or more; the app allows 32.
  const DEFAULT_LIST = 16;
  const MAX_LIST = 32;
  // A list or object read from a page, and the JSON text it can come from.
  const MAX_JSON = 4096;
  const MAX_SOURCE = 2 * 1024 * 1024;

  // Chrome match pattern against a page address: scheme://host/path-and-query.
  function matchesPattern(pattern, address) {
    let url;
    try {
      url = new URL(address);
    } catch {
      return false;
    }
    const split = /^(\*|https?):\/\/([^/]*)(\/.*)$/.exec(pattern);
    if (!split || !/^https?:$/.test(url.protocol)) return false;
    const [, scheme, host, path] = split;
    if (scheme !== "*" && `${scheme}:` !== url.protocol) return false;
    if (host !== "*") {
      if (host.startsWith("*.")) {
        const base = host.slice(2);
        if (url.hostname !== base && !url.hostname.endsWith(`.${base}`)) return false;
      } else if (url.hostname !== host) {
        return false;
      }
    }
    const escaped = path.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*");
    return new RegExp(`^${escaped}$`).test(url.pathname + url.search);
  }

  function clean(text) {
    if (typeof text !== "string") return null;
    const flat = text.replace(/\s+/g, " ").trim();
    return flat === "" ? null : flat.slice(0, MAX_TEXT);
  }

  // A selector can cross into a component with `>>>`: the part before it finds the
  // element whose shadow root holds the rest. Closed roots stay closed.
  function within(doc, selector) {
    const parts = selector.split(">>>").map((part) => part.trim());
    let root = doc;
    for (const part of parts.slice(0, -1)) {
      const host = root.querySelector(part);
      root = host ? host.shadowRoot : null;
      if (!root) return null;
    }
    return { root, selector: parts[parts.length - 1] };
  }

  function find(doc, selector) {
    try {
      const place = within(doc, selector);
      return place ? place.root.querySelector(place.selector) : null;
    } catch {
      return null; // An invalid selector yields no value instead of an error.
    }
  }

  function findAll(doc, selector) {
    try {
      const place = within(doc, selector);
      return place ? place.root.querySelectorAll(place.selector) : [];
    } catch {
      return []; // An invalid selector yields no value instead of an error.
    }
  }

  function isMedia(element) {
    return Boolean(element) && typeof element.paused === "boolean" && "currentTime" in element;
  }

  // Pages can hold several players: muted previews that start on hover or a
  // list of samples, besides the one the user listens to. The main one is the
  // one making sound; then one that can make sound, even paused; then one
  // that plays muted. Ties keep document order.
  function mainMedia(doc, selector) {
    const list = findAll(doc, selector || "video, audio");
    let best = null;
    let bestScore = -1;
    for (const element of list) {
      if (!isMedia(element)) continue;
      const audible = !element.muted && element.volume !== 0;
      const playing = !element.paused;
      const score = (playing && audible ? 4 : 0) + (audible ? 2 : 0) + (playing ? 1 : 0);
      if (score > bestScore) {
        best = element;
        bestScore = score;
      }
    }
    return best;
  }

  function media(element) {
    if (!isMedia(element)) return null;
    const duration = Number.isFinite(element.duration) ? element.duration : null;
    return {
      paused: element.paused,
      current: Math.round(element.currentTime * 1000) / 1000,
      duration: duration === null ? null : Math.round(duration * 1000) / 1000,
      rate: element.playbackRate || 1,
    };
  }

  // What the page publishes through the Media Session API, such as the song
  // that a music site is playing. The last artwork is the largest one sites
  // list. Only web addresses are passed on.
  function session(doc) {
    const view = doc.defaultView;
    const api = view?.navigator?.mediaSession;
    const data = api?.metadata;
    if (!data) return null;
    const title = clean(data.title);
    const artist = clean(data.artist);
    if (title === null && artist === null) return null;
    const pictures = Array.isArray(data.artwork) ? data.artwork : [];
    let artwork = null;
    for (const picture of pictures) {
      try {
        const address = new URL(picture.src, doc.baseURI);
        if (/^https?:$/.test(address.protocol) && address.href.length <= MAX_ADDRESS) artwork = address.href;
      } catch {
        // A picture with an unusable address is skipped.
      }
    }
    return {
      title,
      artist,
      album: clean(data.album),
      artwork,
      state: typeof api.playbackState === "string" ? api.playbackState : null,
    };
  }

  function metaContent(doc, name) {
    for (const tag of doc.getElementsByTagName("meta")) {
      if (tag.getAttribute("name") === name || tag.getAttribute("property") === name) {
        return clean(tag.getAttribute("content"));
      }
    }
    return null;
  }

  function attribute(doc, selector, name) {
    const element = find(doc, selector);
    const value = element ? element.getAttribute(name) : null;
    if (value === null) return null;
    if (name === "href" || name === "src") {
      try {
        const address = new URL(value, doc.baseURI).href;
        return address.length <= MAX_ADDRESS ? address : null;
      } catch {
        return null;
      }
    }
    return clean(value);
  }

  function list(doc, spec) {
    const found = findAll(doc, spec.selector);
    const limit = Math.min(spec.max || DEFAULT_LIST, MAX_LIST);
    const items = [];
    for (const element of found) {
      const value = spec.name ? clean(element.getAttribute(spec.name)) : clean(element.textContent);
      if (value !== null) items.push(value);
      if (items.length === limit) break;
    }
    return items;
  }

  // What a page gave. It can answer anything, so only data passes, and a small
  // amount of it: a whole store of a site is not a status.
  function plain(value) {
    if (typeof value === "string") return clean(value);
    if (typeof value === "boolean" || (typeof value === "number" && Number.isFinite(value))) return value;
    if (value === null || typeof value !== "object") return null;
    try {
      const text = JSON.stringify(value);
      return text.length <= MAX_JSON ? JSON.parse(text) : null;
    } catch {
      return null;
    }
  }

  function walk(value, path) {
    let found = value;
    for (const part of path ? path.split(".") : []) {
      if (found === null || typeof found !== "object" || !Object.hasOwn(found, part)) return null;
      found = found[part];
    }
    return found;
  }

  function parsed(text, path) {
    if (typeof text !== "string" || text.length > MAX_SOURCE) return null;
    try {
      return plain(walk(JSON.parse(text), path));
    } catch {
      return null;
    }
  }

  function stored(doc, spec) {
    try {
      const view = doc.defaultView;
      const text = (spec.area === "session" ? view.sessionStorage : view.localStorage).getItem(spec.key);
      if (text === null) return null;
      return spec.path ? parsed(text, spec.path) : (parsed(text) ?? clean(text));
    } catch {
      return null; // A site can forbid its storage.
    }
  }

  function style(doc, spec) {
    const element = find(doc, spec.selector);
    const value = element ? doc.defaultView.getComputedStyle(element).getPropertyValue(spec.name) : null;
    return clean(value);
  }

  // The paths of the variables a rule reads.
  function variablePaths(extractors) {
    return Object.values(extractors || {})
      .filter((spec) => spec.from === "variable")
      .map((spec) => spec.path);
  }

  // The answers a rule reads: the name of each value, the address and the path.
  function responseEntries(extractors) {
    return Object.entries(extractors || {})
      .filter(([, spec]) => spec.from === "response")
      .map(([name, spec]) => ({ name, url: spec.url, path: spec.path ?? null }));
  }

  // Evaluates every extractor of a rule against a document. `variables` holds what
  // the page's own world answered, by path, and `responses` the answers it kept, by name.
  function extractValues(doc, extractors, variables, responses) {
    const values = {};
    for (const [name, spec] of Object.entries(extractors || {})) {
      switch (spec.from) {
        case "text": {
          const element = find(doc, spec.selector);
          values[name] = element ? clean(element.textContent) : null;
          break;
        }
        case "attr":
          values[name] = attribute(doc, spec.selector, spec.name);
          break;
        case "session":
          values[name] = session(doc);
          break;
        case "meta":
          values[name] = metaContent(doc, spec.name);
          break;
        case "media":
          values[name] = media(mainMedia(doc, spec.selector));
          break;
        case "list":
          values[name] = list(doc, spec);
          break;
        case "variable":
          values[name] = plain(variables ? variables[spec.path] : null);
          break;
        case "response":
          values[name] = plain(responses ? responses[name] : null);
          break;
        case "json": {
          const element = find(doc, spec.selector);
          values[name] = element ? parsed(element.textContent, spec.path) : null;
          break;
        }
        case "storage":
          values[name] = stored(doc, spec);
          break;
        case "style":
          values[name] = style(doc, spec);
          break;
        default:
          values[name] = null;
      }
    }
    return values;
  }

  // True when `next` does not need to be sent after `previous`.
  // Playback position moves on its own, so a media value only counts as
  // changed after a pause, a seek, or a change of duration or speed.
  function sameState(previous, next, elapsedMs) {
    if (!previous) return false;
    const keys = new Set([...Object.keys(previous.values), ...Object.keys(next.values)]);
    if (previous.url !== next.url || previous.title !== next.title) return false;
    for (const key of keys) {
      const a = previous.values[key];
      const b = next.values[key];
      const bothMedia = a && b && typeof a === "object" && typeof b === "object" && "paused" in a && "paused" in b;
      if (!bothMedia) {
        if (JSON.stringify(a) !== JSON.stringify(b)) return false;
        continue;
      }
      if (a.paused !== b.paused || a.duration !== b.duration || a.rate !== b.rate) return false;
      const expected = a.paused ? 0 : (elapsedMs / 1000) * a.rate;
      if (Math.abs(b.current - a.current - expected) > 2) return false;
    }
    return true;
  }

  // The rules for a page. A rule for every site stands aside when another
  // rule was made for this one, or both would report the same tab.
  function rulesForPage(rules, address) {
    const found = rules.filter((rule) => rule.matches.some((pattern) => matchesPattern(pattern, address)));
    const specific = found.filter((rule) => !rule.matches.includes("*://*/*"));
    return specific.length > 0 ? specific : found;
  }

  // What a frame reads: the rules made for the page around it that name this frame.
  function framesForPage(rules, pageAddress, frameAddress) {
    const found = [];
    for (const rule of rulesForPage(rules, pageAddress)) {
      if (rule.frames?.matches.some((pattern) => matchesPattern(pattern, frameAddress))) {
        found.push({ presence: rule.presence, extract: rule.frames.extract });
      }
    }
    return found;
  }

  // Only the names a rule declares for its frames, whatever a frame sent.
  function declaredValues(extract, values) {
    const kept = {};
    for (const name of Object.keys(extract)) kept[name] = values && name in values ? values[name] : null;
    return kept;
  }

  function isPlayer(value) {
    return Boolean(value) && typeof value === "object" && "paused" in value && "current" in value;
  }

  // Joins what several frames said. A page can embed several, some of them empty, so
  // a value beats no value and a player that plays beats one that is paused. A
  // position was true when it was read, so it moves on by the time that passed.
  function joinFrames(entries, now) {
    const joined = {};
    for (const { values, at } of entries) {
      for (const [name, value] of Object.entries(values)) {
        const fresh =
          isPlayer(value) && !value.paused
            ? { ...value, current: value.current + ((now - at) / 1000) * value.rate }
            : value;
        const kept = joined[name];
        const better =
          isPlayer(fresh) && isPlayer(kept) ? kept.paused && !fresh.paused : kept === null || kept === undefined;
        if (better) joined[name] = fresh;
      }
    }
    return joined;
  }

  const api = {
    matchesPattern,
    extractValues,
    variablePaths,
    responseEntries,
    sameState,
    rulesForPage,
    framesForPage,
    declaredValues,
    joinFrames,
  };
  root.RPPExtract = api;
  if (typeof module !== "undefined" && module.exports) module.exports = api;
})(typeof globalThis !== "undefined" ? globalThis : this);
