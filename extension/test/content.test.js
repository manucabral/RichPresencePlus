import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const source = readFileSync(new URL("../content.js", import.meta.url), "utf8");

// A tab: timers run when the test says so, and `chrome` can be cut off the way
// an extension update does it. With `variables`, its rule reads one page variable,
// answered by `world.answer` once the test says the page's world started.
function tab({ variables = false } = {}) {
  const timers = new Set();
  const later = [];
  const world = { answer: null };
  const documentListeners = new Set();
  const answerListeners = new Set();
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
        extractValues: (_, __, read) => (variables ? { episode: read?.["app.episode"] ?? null } : {}),
        variablePaths: () => (variables ? ["app.episode"] : []),
        responseEntries: () => [],
        sameState: () => false,
      },
    },
    location: { href: "https://a.io/" },
    document: {
      title: "A",
      addEventListener: (type, listener) => (type === "rpp-values" ? answerListeners : documentListeners).add(listener),
      removeEventListener: (type, listener) =>
        (type === "rpp-values" ? answerListeners : documentListeners).delete(listener),
      dispatchEvent(event) {
        if (event.type !== "rpp-read" || !world.answer) return;
        for (const listener of answerListeners) listener({ detail: JSON.stringify(world.answer) });
      },
    },
    CustomEvent: class {
      constructor(type, init) {
        this.type = type;
        this.detail = init?.detail;
      }
    },
    addEventListener: (_, listener) => windowListeners.add(listener),
    removeEventListener: (_, listener) => windowListeners.delete(listener),
    setInterval(callback) {
      timers.add(callback);
      return callback;
    },
    clearInterval: (callback) => timers.delete(callback),
    setTimeout: (callback) => later.push(callback),
  };
  const inject = () => new Function(...Object.keys(scope), source)(...Object.values(scope));
  return { inject, scope, sent, runtime, timers, later, world, documentListeners, windowListeners };
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

test("a report waits for the page's world instead of leaving without the variables", async () => {
  const page = tab({ variables: true });
  page.inject();
  await settle();
  assert.equal(page.sent.length, 0, "nothing answered yet");
  assert.equal(page.later.length, 1);
  page.world.answer = { "app.episode": 12 };
  page.later.shift()();
  assert.equal(page.sent.length, 1);
  assert.equal(page.sent[0].values.episode, 12);
});

test("a page whose world never answers is reported after a short wait", async () => {
  const page = tab({ variables: true });
  page.inject();
  await settle();
  let waited = 0;
  while (page.later.length > 0) {
    page.later.shift()();
    waited += 1;
  }
  assert.equal(waited, 20);
  assert.equal(page.sent.length, 1);
  assert.equal(page.sent[0].values.episode, null);
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
