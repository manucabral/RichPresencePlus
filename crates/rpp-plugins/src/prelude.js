// Runs before any plugin code. `/*MEMBERS*/` is replaced by the capabilities
// the manifest was granted; everything else is fixed.
(function () {
  const raw = globalThis.__host;
  delete globalThis.__host;

  // Eval is enabled in the engine only so the script file can be loaded.
  // Remove every way to build code from strings before the plugin runs.
  const blocked = function () {
    throw new Error("code generation is disabled");
  };
  const prototypes = [
    Function.prototype,
    Object.getPrototypeOf(async function () {}),
    Object.getPrototypeOf(function* () {}),
    Object.getPrototypeOf(async function* () {}),
  ];
  for (const proto of prototypes) {
    Object.defineProperty(proto, "constructor", {
      value: blocked,
      writable: false,
      configurable: false,
    });
  }
  Object.defineProperty(globalThis, "Function", {
    value: blocked,
    writable: false,
    configurable: false,
  });
  Object.defineProperty(globalThis, "eval", {
    value: undefined,
    writable: false,
    configurable: false,
  });

  const call = (kind, args) =>
    JSON.parse(raw(kind, JSON.stringify(args === undefined ? null : args)));
  Object.defineProperty(globalThis, "host", {
    value: Object.freeze({ /*MEMBERS*/ }),
    writable: false,
    configurable: false,
  });
  // `console.log/warn/error` print to the developer tools. They need no
  // permission and nothing is written to disk.
  const show = (value) => {
    if (typeof value === "string") return value;
    try {
      const text = JSON.stringify(value);
      return text === undefined ? String(value) : text;
    } catch (error) {
      return String(value);
    }
  };
  const print = (level) =>
    function (...parts) {
      call("console", { level, text: parts.map(show).join(" ") });
    };
  Object.defineProperty(globalThis, "console", {
    value: Object.freeze({ log: print("log"), warn: print("warn"), error: print("error") }),
    writable: false,
    configurable: false,
  });

  // `t(key, values)` returns the text of the user's language for a key and
  // fills `{name}` placeholders. A key with no text is returned as written.
  Object.defineProperty(globalThis, "t", {
    value: function t(key, values) {
      const table = globalThis.__strings;
      const found = table && typeof table[key] === "string" ? table[key] : String(key);
      if (values === undefined || values === null) return found;
      return found.replace(/\{([A-Za-z0-9_]+)\}/g, (whole, name) =>
        Object.prototype.hasOwnProperty.call(values, name) ? String(values[name]) : whole,
      );
    },
    writable: false,
    configurable: false,
  });
  // Small helpers that nearly every presence needs. They are fixed and
  // frozen like `t`, so a script cannot change what another script sees.
  const define = (name, value) =>
    Object.defineProperty(globalThis, name, { value, writable: false, configurable: false });

  // `cut(text, max)`: the text shortened to `max` characters (128 by default,
  // the most Discord shows), ending in an ellipsis when it was longer.
  define("cut", function cut(text, max) {
    const limit = max === undefined ? 128 : max;
    if (typeof text !== "string") return null;
    return text.length > limit ? text.slice(0, limit - 1) + "…" : text;
  });

  // `parseUrl(address)`: `{ host, path, query }` for a web address, or null.
  // `query` maps each parameter to its decoded value. The engine has no URL class.
  define("parseUrl", function parseUrl(address) {
    const found = /^https?:\/\/([^/?#]+)([^?#]*)(?:\?([^#]*))?/.exec(address || "");
    if (!found) return null;
    const query = {};
    for (const pair of (found[3] || "").split("&")) {
      if (pair === "") continue;
      const [key, value = ""] = pair.split("=");
      try {
        query[decodeURIComponent(key)] = decodeURIComponent(value.replace(/\+/g, " "));
      } catch (error) {
        query[key] = value;
      }
    }
    return { host: found[1], path: found[2] || "/", query };
  });

  // `bestPage(pages)`: the page the user is most likely looking at. A page
  // whose player is running wins, then any page with a player, then the first.
  define("bestPage", function bestPage(pages) {
    const list = Array.isArray(pages) ? pages : [];
    const player = (page) => page.values && page.values.player;
    return (
      list.find((page) => player(page) && !player(page).paused) ||
      list.find((page) => player(page)) ||
      list[0] ||
      null
    );
  });

  // `playback(player, receivedAt)`: `{ playing, start, end }` for a `media`
  // value of a page. `start` and `end` are Unix seconds that make Discord draw
  // a progress bar; they are null while paused or when the length is unknown.
  // Playback positions are valid when the application received the page, so
  // pass `page.received_at`.
  // `parseTime(text)`: seconds in a clock text such as `1:02:03`, `4:05` or `30`,
  // for players that only show their time as text.
  define("parseTime", function parseTime(text) {
    if (typeof text !== "string") return null;
    const found = /^\s*(?:(\d+):)?(?:(\d+):)?(\d+)\s*$/.exec(text);
    if (!found) return null;
    const parts = found.slice(1).filter((part) => part !== undefined).map(Number);
    return parts.reduce((total, part) => total * 60 + part, 0);
  });

  // `progress(current, duration, now)`: the progress bar for a position and a length
  // in seconds, measured at `now`.
  define("progress", function progress(current, duration, now) {
    const start = Math.round(now - current);
    return { start, end: start + Math.round(duration) };
  });

  // `rotate(items, seconds, now)`: one of the items, the next one every `seconds`
  // seconds of `now`. Empty items are skipped, so optional values can be passed as
  // they are. The change shows on the next read, so `seconds` below the manifest's
  // `interval` gains nothing.
  define("rotate", function rotate(items, seconds, now) {
    const list = (Array.isArray(items) ? items : []).filter(
      (item) => item !== null && item !== undefined && item !== "",
    );
    if (list.length === 0) return null;
    const step = typeof seconds === "number" && seconds >= 1 ? seconds : 30;
    const turn = Math.floor((typeof now === "number" ? now : 0) / step);
    return list[((turn % list.length) + list.length) % list.length];
  });

  // `format(template, values)`: a text the person wrote, such as "{title} - {channel}",
  // with each `{name}` filled from `values`. A missing value leaves nothing, and a
  // separator left at either end goes with it. A name that is not in `values` stays
  // as written, so a typo shows.
  define("format", function format(template, values) {
    if (typeof template !== "string") return null;
    const given = values && typeof values === "object" ? values : {};
    const filled = template.replace(/\{([A-Za-z0-9_]+)\}/g, (whole, name) => {
      if (!Object.prototype.hasOwnProperty.call(given, name)) return whole;
      const value = given[name];
      return value === null || value === undefined ? "" : String(value);
    });
    // "a - - b" keeps one separator; "/" and ":" only go at the ends, since an
    // address in a value has them in its middle.
    const text = filled
      .replace(/\s+/g, " ")
      .replace(/(\s*[-–|•,]\s*)(?:[-–|•,]\s*)+/g, "$1")
      .replace(/^[\s\-–|•:,/]+|[\s\-–|•:,/]+$/g, "");
    return text === "" ? null : text;
  });

  // `icon(name)`: the address of a small picture every presence can use, or null
  // for a name not in the list. They live on the project's site, so they work with
  // any Discord application.
  const ICONS = ["play", "pause", "stop", "live", "repeat", "repeat-one", "shuffle"];
  define("icon", function icon(name) {
    return ICONS.includes(name) ? `https://manucabral.github.io/RichPresencePlus/icons/${name}.png` : null;
  });

  define("playback", function playback(player, receivedAt) {
    const idle = { playing: false, start: null, end: null };
    if (!player || player.paused) return idle;
    const rate = player.rate > 0 ? player.rate : 1;
    const start = Math.round(receivedAt / 1000 - player.current / rate);
    // At 2x a 4-minute song ends in 2 minutes of real time.
    const end = player.duration > 0 ? start + Math.floor(player.duration / rate) : null;
    return { playing: true, start, end };
  });
})();
