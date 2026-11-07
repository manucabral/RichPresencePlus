import assert from "node:assert/strict";
import { test } from "node:test";
import {
  choosePairing,
  HOST,
  HOST_RETRY_MS,
  nextDelay,
  openSites,
  originsOf,
  parseHostAnswer,
  parsePairingCode,
  rulesForOrigin,
  samePairing,
  shouldAskHost,
  toOrigin,
  usesHost,
} from "../pairing.js";

const TOKEN = "0123456789abcdef".repeat(4);

test("a pairing code has a port and a 64-character token", () => {
  assert.deepEqual(parsePairingCode(`38417:${TOKEN}`), { port: 38417, token: TOKEN });
  assert.deepEqual(parsePairingCode(`  38417:${TOKEN.toUpperCase()}\n`), { port: 38417, token: TOKEN });
  for (const bad of ["", "38417", `0:${TOKEN}`, `70000:${TOKEN}`, "38417:short", `38417:${TOKEN}0`]) {
    assert.equal(parsePairingCode(bad), null, bad);
  }
});

test("reconnection waits grow to a ceiling", () => {
  assert.deepEqual([0, 1, 2, 3, 4, 5, 6, 20].map(nextDelay), [1000, 2000, 4000, 8000, 16000, 30000, 30000, 30000]);
});

test("host permissions drop the path and keep the host", () => {
  assert.equal(toOrigin("https://www.example.com/watch*"), "https://www.example.com/*");
  assert.equal(toOrigin("*://*.example.com/a/b"), "*://*.example.com/*");
  assert.equal(toOrigin("nonsense"), null);
  const rules = [{ matches: ["*://*.a.io/x*", "*://*.a.io/y*"] }, { matches: ["https://b.io/*"] }];
  assert.deepEqual(originsOf(rules), ["*://*.a.io/*", "https://b.io/*"]);
});

test("the host's answer gives a pairing only when it is complete", () => {
  assert.deepEqual(parseHostAnswer({ type: "pairing", port: 38417, token: TOKEN }), {
    port: 38417,
    token: TOKEN,
    auto: true,
  });
  for (const bad of [
    null,
    { type: "absent" },
    { type: "refused" },
    { type: "pairing", port: 0, token: TOKEN },
    { type: "pairing", port: 70000, token: TOKEN },
    { type: "pairing", port: "38417", token: TOKEN },
    { type: "pairing", port: 1, token: TOKEN.toUpperCase() },
    { type: "pairing", port: 1, token: "short" },
    { type: "pairing", port: 1 },
  ]) {
    assert.equal(parseHostAnswer(bad), null, JSON.stringify(bad));
  }
});

test("the host is asked without a pairing, and again after a while", () => {
  const stored = { port: 1, token: TOKEN };
  assert.equal(shouldAskHost(undefined, 1000, 1000), true);
  assert.equal(shouldAskHost(stored, 1000, 1000 + HOST_RETRY_MS - 1), false);
  assert.equal(shouldAskHost(stored, 1000, 1000 + HOST_RETRY_MS), true);
  // A worker that just started has never asked.
  assert.equal(shouldAskHost(stored, 0, Date.now()), true);
});

test("the app's answer wins over a stored pairing", () => {
  const typed = { port: 1, token: TOKEN };
  const answer = { type: "pairing", port: 2, token: TOKEN };
  assert.deepEqual(choosePairing(typed, answer), { port: 2, token: TOKEN, auto: true });
  assert.equal(choosePairing(typed, null), typed);
  assert.equal(choosePairing(typed, { type: "absent" }), typed);
  assert.equal(choosePairing(undefined, null), null);
});

test("pairings are the same when port and token are", () => {
  assert.equal(samePairing({ port: 1, token: TOKEN }, { port: 1, token: TOKEN, auto: true }), true);
  assert.equal(samePairing({ port: 1, token: TOKEN }, { port: 2, token: TOKEN }), false);
  assert.equal(samePairing(null, { port: 1, token: TOKEN }), false);
});

test("the host name matches the desktop app's", () => {
  assert.equal(HOST, "com.manucabral.richpresenceplus");
});

test("a page's origin is its scheme and host, for web pages", async () => {
  const { pageOrigin, isOpenRule } = await import("../pairing.js");
  assert.equal(pageOrigin("https://www.a.io/watch?v=1"), "https://www.a.io/*");
  assert.equal(pageOrigin("http://a.io/"), "http://a.io/*");
  assert.equal(pageOrigin("chrome://extensions"), null);
  assert.equal(pageOrigin("not an address"), null);
  assert.ok(isOpenRule({ matches: ["*://*/*"] }));
  assert.ok(!isOpenRule({ matches: ["*://*.a.io/*"] }));
});

test("a presence for every site gets only the sites chosen for it", () => {
  const chosen = ["https://a.com/*", "https://gone.com/*"];
  const granted = ["https://a.com/*", "https://netflix.com/*", "*://*/*"];
  // Allowed for another presence does not count, and a withdrawn site is left out.
  assert.deepEqual(openSites(chosen, granted), ["https://a.com/*"]);
  assert.deepEqual(openSites(["*://*/*"], granted), []);
  assert.deepEqual(openSites([], granted), []);
});

test("a page only receives a rule for every site where it was chosen", () => {
  const open = { presence: "Web Media", matches: ["*://*/*"] };
  const video = { presence: "Video", matches: ["*://v.com/*"] };
  const sites = ["https://a.com/*"];
  assert.deepEqual(rulesForOrigin([open, video], "https://a.com/*", sites), [open, video]);
  assert.deepEqual(rulesForOrigin([open, video], "https://v.com/*", sites), [video]);
  assert.deepEqual(rulesForOrigin([open], null, sites), []);
});

test("a host read by a presence for some sites keeps its permission", () => {
  const rules = [
    { presence: "Web Media", matches: ["*://*/*"] },
    { presence: "Video", matches: ["*://www.v.com/watch*"] },
  ];
  assert.equal(usesHost(rules, "https://www.v.com/other"), true);
  assert.equal(usesHost(rules, "https://a.com/"), false);
  assert.equal(usesHost(rules, "not an address"), false);
});

test("the sites of a rule include its frames", () => {
  const rule = {
    matches: ["*://www.example.com/*"],
    frames: { matches: ["*://player.example.net/embed/*"], extract: {} },
  };
  assert.deepEqual(originsOf([rule]), ["*://www.example.com/*", "*://player.example.net/*"]);
});
