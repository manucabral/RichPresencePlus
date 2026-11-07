import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { JSDOM } from "jsdom";

// extract.js is a plain script; load it the way a content script would run.
const api = {};
new Function("globalThis", readFileSync(new URL("../extract.js", import.meta.url), "utf8"))(api);
const {
  matchesPattern,
  extractValues,
  sameState,
  rulesForPage,
  variablePaths,
  framesForPage,
  declaredValues,
  joinFrames,
} = api.RPPExtract;

const page = (html) => new JSDOM(html, { url: "https://www.example.com/watch?v=1" }).window.document;

test("match patterns follow Chrome's rules", () => {
  const yes = [
    ["*://*.example.com/*", "https://www.example.com/watch?v=1"],
    ["*://*.example.com/*", "http://example.com/"],
    ["https://www.example.com/watch*", "https://www.example.com/watch?v=1"],
    ["*://*/*", "https://anything.io/x"],
  ];
  const no = [
    ["https://www.example.com/watch*", "https://www.example.com/home"],
    ["https://*.example.com/*", "http://www.example.com/"],
    ["*://*.example.com/*", "https://notexample.com/"],
    ["*://*.example.com/*", "https://example.com.evil.io/"],
    ["*://*/*", "file:///etc/passwd"],
    ["*://*/*", "not a url"],
    ["ftp://a.io/*", "https://a.io/"],
  ];
  for (const [pattern, url] of yes) assert.ok(matchesPattern(pattern, url), `${pattern} ${url}`);
  for (const [pattern, url] of no) assert.ok(!matchesPattern(pattern, url), `${pattern} ${url}`);
});

test("text, attribute and meta values are read and cleaned", () => {
  const doc = page(`<head><meta property="og:title" content=" Big  Buck "></head>
    <body><h1>  Hello
      world </h1><a class="owner" href="/channel/42">Owner</a></body>`);
  const values = extractValues(doc, {
    title: { from: "text", selector: "h1" },
    channel: { from: "attr", selector: "a.owner", name: "href" },
    og: { from: "meta", name: "og:title" },
    missing: { from: "text", selector: ".nothing" },
    broken: { from: "text", selector: "((" },
  });
  assert.deepEqual(values, {
    title: "Hello world",
    channel: "https://www.example.com/channel/42",
    og: "Big Buck",
    missing: null,
    broken: null,
  });
});

test("media state is read from the first media element or a selector", () => {
  const doc = page(`<video id="a"></video><audio id="b"></audio>`);
  const video = doc.getElementById("a");
  Object.defineProperties(video, {
    paused: { value: false },
    currentTime: { value: 12.3456 },
    duration: { value: 600 },
    playbackRate: { value: 1.5 },
  });
  const first = extractValues(doc, { player: { from: "media" } });
  assert.deepEqual(first.player, { paused: false, current: 12.346, duration: 600, rate: 1.5 });
  const none = extractValues(page("<p>no media</p>"), { player: { from: "media" } });
  assert.equal(none.player, null);
  const wrong = extractValues(doc, { player: { from: "media", selector: "#missing" } });
  assert.equal(wrong.player, null);
});

test("the player making sound wins over muted previews", () => {
  const doc = page(`<video id="preview"></video><video id="main"></video><audio id="other"></audio>`);
  const set = (id, props) =>
    Object.defineProperties(
      doc.getElementById(id),
      Object.fromEntries(Object.entries(props).map(([key, value]) => [key, { value, configurable: true }])),
    );
  // A muted sample autoplays before the real player in the page.
  set("preview", { paused: false, muted: true, currentTime: 3, duration: 786 });
  set("main", { paused: false, muted: false, volume: 1, currentTime: 61, duration: 185 });
  assert.equal(extractValues(doc, { player: { from: "media" } }).player.duration, 185);
  // Paused, the real player still beats the muted preview.
  set("main", { paused: true, muted: false, volume: 1, currentTime: 61, duration: 185 });
  assert.equal(extractValues(doc, { player: { from: "media" } }).player.duration, 185);
  // With a selector, only its matches compete.
  assert.equal(extractValues(doc, { player: { from: "media", selector: "#preview" } }).player.duration, 786);
  assert.equal(extractValues(doc, { player: { from: "media", selector: "((" } }).player, null);
});

test("unknown extractor kinds yield null instead of running anything", () => {
  const values = extractValues(page("<p>x</p>"), { x: { from: "script", code: "alert(1)" } });
  assert.deepEqual(values, { x: null });
});

const state = (current, extra = {}) => ({
  url: "https://a.io/v",
  title: "T",
  values: { player: { paused: false, current, duration: 100, rate: 1, ...extra } },
});

test("playback that advances normally is not sent again", () => {
  assert.ok(sameState(state(10), state(12), 2000));
  assert.ok(sameState(state(10), state(12.8), 2000));
});

test("seeks, pauses and changes of duration or speed are sent", () => {
  assert.ok(!sameState(state(10), state(60), 2000));
  assert.ok(!sameState(state(10), state(10, { paused: true }), 2000));
  assert.ok(!sameState(state(10), state(12, { duration: 200 }), 2000));
  assert.ok(!sameState(state(10), state(12, { rate: 2 }), 2000));
  assert.ok(!sameState(null, state(10), 0));
});

test("a paused video stays unchanged until it moves", () => {
  const paused = (current) => state(current, { paused: true });
  assert.ok(sameState(paused(10), paused(10), 5000));
  assert.ok(!sameState(paused(10), paused(40), 5000));
});

test("a new address, title or text value is sent", () => {
  const base = state(10);
  assert.ok(!sameState(base, { ...state(12), title: "Other" }, 2000));
  assert.ok(!sameState(base, { ...state(12), url: "https://a.io/w" }, 2000));
  const text = (value) => ({ url: "u", title: "t", values: { title: value } });
  assert.ok(sameState(text("a"), text("a"), 1000));
  assert.ok(!sameState(text("a"), text("b"), 1000));
});

test("the media session is read when the page publishes one", () => {
  const doc = page("<body></body>");
  const spec = { now: { from: "session" } };
  assert.equal(extractValues(doc, spec).now, null);

  Object.defineProperty(doc.defaultView.navigator, "mediaSession", {
    configurable: true,
    value: {
      playbackState: "playing",
      metadata: {
        title: "  A  Song ",
        artist: "Someone",
        album: "",
        artwork: [
          { src: "/small.jpg", sizes: "96x96" },
          { src: "javascript:alert(1)" },
          { src: "https://img.example.com/large.jpg", sizes: "512x512" },
        ],
      },
    },
  });
  assert.deepEqual(extractValues(doc, spec).now, {
    title: "A Song",
    artist: "Someone",
    album: null,
    artwork: "https://img.example.com/large.jpg",
    state: "playing",
  });

  // Without a title or an artist there is nothing worth sending.
  doc.defaultView.navigator.mediaSession.metadata = { title: " ", artist: "", artwork: [] };
  assert.equal(extractValues(doc, spec).now, null);
});

test("an unusable artwork address is dropped, not sent", () => {
  const doc = page("<body></body>");
  Object.defineProperty(doc.defaultView.navigator, "mediaSession", {
    configurable: true,
    value: {
      metadata: { title: "T", artist: "A", artwork: [{ src: "data:image/png;base64,AAAA" }, { src: "http://[bad" }] },
    },
  });
  const value = extractValues(doc, { now: { from: "session" } }).now;
  assert.equal(value.artwork, null);
  assert.equal(value.state, null);
});

test("a rule for every site stands aside for one made for the page", () => {
  const open = { presence: "Web", matches: ["*://*/*"] };
  const site = { presence: "Site", matches: ["*://a.io/*"] };
  assert.deepEqual(rulesForPage([open, site], "https://a.io/x"), [site]);
  assert.deepEqual(rulesForPage([open, site], "https://b.io/"), [open]);
  assert.deepEqual(rulesForPage([site], "https://b.io/"), []);
});

test("addresses a page makes too long are left out", () => {
  const long = `https://cdn.example.com/${"x".repeat(5000)}`;
  const doc = page(`<a class="owner" href="${long}">o</a><a class="short" href="/a">o</a>`);
  const values = extractValues(doc, {
    long: { from: "attr", selector: "a.owner", name: "href" },
    fine: { from: "attr", selector: "a.short", name: "href" },
  });
  assert.equal(values.long, null);
  assert.equal(values.fine, "https://www.example.com/a");
});

const elements = (items) => ({
  querySelectorAll: () => items.map((text) => ({ textContent: text, getAttribute: () => `/${text}` })),
  querySelector: () => null,
});

test("a list reads every match, up to its limit", () => {
  const doc = elements(["Action", "  Drama ", "", "Comedy"]);
  assert.deepEqual(extractValues(doc, { tags: { from: "list", selector: "li" } }).tags, ["Action", "Drama", "Comedy"]);
  assert.deepEqual(extractValues(doc, { tags: { from: "list", selector: "li", max: 1 } }).tags, ["Action"]);
  assert.deepEqual(extractValues(doc, { tags: { from: "list", selector: "a", name: "href", max: 2 } }).tags, [
    "/Action",
    "/ Drama",
  ]);
  const many = elements(Array.from({ length: 50 }, (_, index) => `item ${index}`));
  assert.equal(extractValues(many, { tags: { from: "list", selector: "li", max: 200 } }).tags.length, 32);
  assert.deepEqual(
    extractValues(
      {
        querySelectorAll: () => {
          throw new Error("bad");
        },
      },
      { tags: { from: "list", selector: "[" } },
    ).tags,
    [],
  );
});

test("a variable takes only plain values, whatever the page answers", () => {
  const extract = {
    a: { from: "variable", path: "a" },
    b: { from: "variable", path: "b" },
    c: { from: "variable", path: "c" },
  };
  const doc = elements([]);
  assert.deepEqual(extractValues(doc, extract, { a: " text ", b: 5, c: true }), { a: "text", b: 5, c: true });
  assert.deepEqual(extractValues(doc, extract, { a: { title: "x" }, b: ["y"], c: Infinity }), {
    a: { title: "x" },
    b: ["y"],
    c: null,
  });
  assert.equal(extractValues(doc, extract, { a: { long: "x".repeat(5000) } }).a, null);
  assert.deepEqual(extractValues(doc, extract, null), { a: null, b: null, c: null });
  assert.equal(extractValues(doc, extract, { a: "x".repeat(9000) }).a.length, 512);
});

test("a rule says which variables it needs", () => {
  assert.deepEqual(variablePaths({ a: { from: "variable", path: "x.y" }, b: { from: "text", selector: "h1" } }), [
    "x.y",
  ]);
  assert.deepEqual(variablePaths({ b: { from: "text", selector: "h1" } }), []);
});

const framed = {
  presence: "Anime",
  matches: ["*://www.example.com/*"],
  frames: { matches: ["*://player.example.net/*"], extract: { player: { from: "media" } } },
};

test("a frame reads only inside the page its rule was made for", () => {
  const rules = [framed, { presence: "Other", matches: ["*://www.other.com/*"] }];
  const frame = "https://player.example.net/embed/1";
  assert.deepEqual(framesForPage(rules, "https://www.example.com/watch", frame), [
    { presence: "Anime", extract: framed.frames.extract },
  ]);
  // The same player embedded in another site gives it nothing.
  assert.deepEqual(framesForPage(rules, "https://www.other.com/", frame), []);
  assert.deepEqual(framesForPage(rules, "https://www.example.com/watch", "https://ads.example.org/"), []);
});

test("only the names a rule declares pass", () => {
  const extract = { player: { from: "media" } };
  assert.deepEqual(declaredValues(extract, { player: 1, title: "from a frame", url: "x" }), { player: 1 });
  assert.deepEqual(declaredValues(extract, undefined), { player: null });
});

test("frames join: a value beats none, a player that plays beats a paused one", () => {
  const paused = { paused: true, current: 5, duration: 100, rate: 1 };
  const playing = { paused: false, current: 10, duration: 100, rate: 1 };
  const ad = { values: { player: null, title: "Real" }, at: 1000 };
  assert.deepEqual(
    joinFrames(
      [ad, { values: { player: paused, title: null }, at: 1000 }, { values: { player: playing }, at: 1000 }],
      1000,
    ),
    { player: playing, title: "Real" },
  );
});

test("the position of a frame moves on with the time that passed", () => {
  const playing = { paused: false, current: 10, duration: 100, rate: 2 };
  assert.equal(joinFrames([{ values: { player: playing }, at: 1000 }], 4000).player.current, 16);
  const paused = { ...playing, paused: true };
  assert.equal(joinFrames([{ values: { player: paused }, at: 1000 }], 4000).player.current, 10);
});

test("JSON written in the page is read by its path", () => {
  const doc =
    page(`<script id="data" type="application/json">{"props":{"title":"A movie","year":1999,"cast":["A","B"]}}</script>
    <script id="broken" type="application/json">{ not json</script>`);
  const read = (spec) => extractValues(doc, { v: { from: "json", ...spec } }).v;
  assert.equal(read({ selector: "#data", path: "props.title" }), "A movie");
  assert.equal(read({ selector: "#data", path: "props.year" }), 1999);
  assert.deepEqual(read({ selector: "#data", path: "props.cast" }), ["A", "B"]);
  assert.equal(read({ selector: "#data", path: "props.missing.more" }), null);
  assert.equal(read({ selector: "#data", path: "constructor" }), null);
  assert.equal(read({ selector: "#broken" }), null);
  assert.equal(read({ selector: "#nothing" }), null);
});

test("the storage of a site is read by its key", () => {
  const dom = new JSDOM("", { url: "https://www.example.com/" });
  const { document, localStorage, sessionStorage } = dom.window;
  localStorage.setItem("player", '{"track":{"title":"A song"}}');
  localStorage.setItem("volume", "0.5");
  localStorage.setItem("name", "plain text");
  sessionStorage.setItem("queue", "[1,2]");
  const read = (spec) => extractValues(document, { v: { from: "storage", ...spec } }).v;
  assert.equal(read({ key: "player", path: "track.title" }), "A song");
  assert.equal(read({ key: "volume" }), 0.5);
  assert.equal(read({ key: "name" }), "plain text");
  assert.deepEqual(read({ key: "queue", area: "session" }), [1, 2]);
  assert.equal(read({ key: "missing" }), null);
});

test("a computed style is read", () => {
  const doc = page(`<style>.poster { color: red; }</style><div class="poster"></div>`);
  assert.equal(extractValues(doc, { v: { from: "style", selector: ".poster", name: "color" } }).v, "rgb(255, 0, 0)");
  assert.equal(extractValues(doc, { v: { from: "style", selector: ".none", name: "color" } }).v, null);
});

test("a selector crosses into a component", () => {
  const doc = page(`<x-player id="host"></x-player><x-closed id="closed"></x-closed>`);
  doc.getElementById("host").attachShadow({ mode: "open" }).innerHTML = "<h2>Inside</h2><li>One</li><li>Two</li>";
  doc.getElementById("closed").attachShadow({ mode: "closed" }).innerHTML = "<h2>Private</h2>";
  const read = (spec) => extractValues(doc, { v: spec }).v;
  assert.equal(read({ from: "text", selector: "#host >>> h2" }), "Inside");
  assert.deepEqual(read({ from: "list", selector: "#host >>> li" }), ["One", "Two"]);
  assert.equal(read({ from: "text", selector: "#closed >>> h2" }), null);
  assert.equal(read({ from: "text", selector: "#none >>> h2" }), null);
});
