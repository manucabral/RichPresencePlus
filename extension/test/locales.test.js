// Every text the popup shows exists in English and Spanish.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { describeState, elapsed, presenceNames, ruleForUrl } from "../status.js";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const en = JSON.parse(read("../_locales/en/messages.json"));
const es = JSON.parse(read("../_locales/es/messages.json"));
const manifest = JSON.parse(read("../manifest.json"));

test("both languages have the same keys and no empty text", () => {
  assert.deepEqual(Object.keys(es).sort(), Object.keys(en).sort());
  for (const [key, entry] of [...Object.entries(en), ...Object.entries(es)]) {
    assert.ok(/^[A-Za-z0-9_]+$/.test(key), key);
    assert.ok(entry.message.trim().length > 0, key);
  }
});

test("every key the popup and the manifest use exists", () => {
  const used = new Set();
  for (const match of read("../popup.html").matchAll(/data-i18n="([^"]+)"/g)) used.add(match[1]);
  for (const state of [{ connected: true }, { untrusted: true }, { paired: true }, { host: "missing" }, {}]) {
    used.add(describeState({ connected: false, untrusted: false, paired: false, host: "unknown", ...state }).key);
  }
  for (const match of JSON.stringify(manifest).matchAll(/__MSG_([A-Za-z0-9_]+)__/g)) used.add(match[1]);
  for (const match of read("../popup.js").matchAll(/(?<![\w.])t\("([A-Za-z0-9_]+)"/g)) used.add(match[1]);
  for (const at of [0, 5 * 60000, 3 * 3600000]) used.add(elapsed(0, at).key);
  for (const key of used) assert.ok(key in en, `missing in en: ${key}`);
  assert.equal(manifest.default_locale, "en");
});

test("the status picks a message and a color", () => {
  const base = { connected: false, untrusted: false, paired: false, host: "unknown" };
  assert.deepEqual(describeState({ ...base, connected: true }), { key: "connected", tone: "on" });
  assert.deepEqual(describeState({ ...base, untrusted: true }), { key: "untrusted", tone: "bad" });
  assert.deepEqual(describeState({ ...base, paired: true }), { key: "waiting", tone: "warn" });
  assert.deepEqual(describeState({ ...base, host: "missing" }), { key: "missingHost", tone: "bad" });
  assert.deepEqual(describeState(base), { key: "looking", tone: "" });
});

test("a page finds the presence whose patterns match it", () => {
  const rules = [
    { presence: "YouTube", matches: ["*://www.youtube.com/*"] },
    { presence: "YouTube Music", matches: ["*://music.youtube.com/*"] },
  ];
  const matches = (pattern, url) => url.startsWith(pattern.replace("*://", "https://").replace("/*", "/"));
  assert.equal(ruleForUrl(rules, "https://music.youtube.com/watch", matches).presence, "YouTube Music");
  assert.equal(ruleForUrl(rules, "https://example.com/", matches), null);
  assert.equal(ruleForUrl(rules, undefined, matches), null);
  const open = { presence: "Web", matches: ["*://*/*"] };
  const named = ruleForUrl([open, ...rules], "https://music.youtube.com/watch", matches);
  assert.notEqual(named.presence, "Web");
});

test("connection times read in minutes or hours, and names are listed once", () => {
  assert.deepEqual(elapsed(1000, 30_000), { key: "forMoment", count: 0 });
  assert.deepEqual(elapsed(0, 12 * 60000), { key: "forMinutes", count: 12 });
  assert.deepEqual(elapsed(0, 125 * 60000), { key: "forHours", count: 2 });
  assert.deepEqual(elapsed(5000, 0), { key: "forMoment", count: 0 });
  assert.deepEqual(presenceNames([{ presence: "b" }, { presence: "a" }, { presence: "b" }]), ["a", "b"]);
});
