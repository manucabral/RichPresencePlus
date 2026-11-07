import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const source = readFileSync(new URL("../content.js", import.meta.url), "utf8");

// A tab: timers run when the test says so, and `chrome` can be cut off the way
// an extension update does it.
function tab() {
  const timers = new Set();
  const documentListeners = new Set();
  const windowListeners = new Set();
  const sent = [];
  const runtime = {
    id: "extension",
    onMessage: { addListener() {} },
    async sendMessage(message) {
      if (message.type === "rules") return [{ presence: "Demo", matches: ["*"], extract: {} }];
      sent.push(message);
      return undefined;
    },
  };
  const scope = {
    chrome: { runtime },
    globalThis: {
      RPPExtract: {
        rulesForPage: (rules) => rules,
        extractValues: () => ({}),
        variablePaths: () => [],
        sameState: () => false,
      },
    },
    location: { href: "https://a.io/" },
    document: {
      title: "A",
      addEventListener: (_, listener) => documentListeners.add(listener),
      removeEventListener: (_, listener) => documentListeners.delete(listener),
    },
    addEventListener: (_, listener) => windowListeners.add(listener),
    removeEventListener: (_, listener) => windowListeners.delete(listener),
    setInterval(callback) {
      timers.add(callback);
      return callback;
    },
    clearInterval: (callback) => timers.delete(callback),
  };
  const inject = () => new Function(...Object.keys(scope), source)(...Object.values(scope));
  return { inject, scope, sent, runtime, timers, documentListeners, windowListeners };
}

const settle = () => new Promise((resolve) => setImmediate(resolve));

test("a second injection in a live tab changes nothing", async () => {
  const page = tab();
  page.inject();
  await settle();
  const count = page.sent.length;
  assert.equal(count, 1);
  page.inject();
  await settle();
  assert.equal(page.timers.size, 1);
  assert.equal(page.sent.length, count);
});

test("a script cut off by an update stops its timer and listeners", async () => {
  const page = tab();
  page.inject();
  await settle();
  assert.equal(page.timers.size, 1);
  assert.equal(page.documentListeners.size, 1);
  delete page.runtime.id;
  page.runtime.sendMessage = () => {
    throw new Error("Extension context invalidated.");
  };
  for (const run of page.timers) run();
  assert.equal(page.timers.size, 0);
  assert.equal(page.documentListeners.size, 0);
  assert.equal(page.windowListeners.size, 0);
  assert.equal(page.scope.globalThis.__rppContent, null);
});

test("an updated script replaces the one that was cut off", async () => {
  const page = tab();
  page.inject();
  await settle();
  const old = page.scope.globalThis.__rppContent;
  old.alive = () => false;
  page.inject();
  await settle();
  assert.equal(page.timers.size, 1);
  assert.equal(page.sent.length, 2);
  assert.notEqual(page.scope.globalThis.__rppContent, old);
});
