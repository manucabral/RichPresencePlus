import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import vm from "node:vm";

const source = readFileSync(new URL("../responses.js", import.meta.url), "utf8");

// Lets the reading of every answer finish.
const settle = () => new Promise((resolve) => setTimeout(resolve, 20));

// A page with `responses.js` in it. `answers` is what each address answers:
// `{ body, type }`. `timers` holds what `setTimeout` was given, to run by hand.
function page(answers = {}) {
  const document = new EventTarget();
  const timers = [];
  const calls = [];
  async function realFetch(input, init) {
    calls.push({ input, init, self: this });
    const { body = "", type = "application/json" } = answers[input] ?? {};
    const response = new Response(body, { headers: { "content-type": type } });
    Object.defineProperty(response, "url", { value: input });
    return response;
  }
  class Request {
    open(_method, url) {
      this.opened = (this.opened ?? 0) + 1;
      this.responseURL = url;
      return "opened";
    }
    addEventListener(type, listener) {
      this.listeners = [...(this.listeners ?? []), { type, listener }];
    }
    getResponseHeader() {
      return this.type ?? "application/json";
    }
    load() {
      for (const { listener } of this.listeners ?? []) listener.call(this);
    }
  }
  const context = {
    document,
    location: { href: "https://www.example.com/", origin: "https://www.example.com" },
    fetch: realFetch,
    XMLHttpRequest: Request,
    CustomEvent,
    TextDecoder,
    URL,
    queueMicrotask,
    setTimeout: (callback) => timers.push(callback),
  };
  context.window = context;
  vm.createContext(context);
  vm.runInContext(source, context);
  const ask = (presence, entries) => {
    let answer;
    const take = (event) => {
      answer = JSON.parse(event.detail);
    };
    document.addEventListener("rpp-answers", take);
    document.dispatchEvent(new CustomEvent("rpp-responses", { detail: JSON.stringify({ presence, entries }) }));
    document.removeEventListener("rpp-answers", take);
    return answer;
  };
  return { context, ask, timers, calls, Request };
}

const API = "https://api.example.com/v1/player/7";
const TITLE = [{ name: "title", url: "*://api.example.com/v1/player/*", path: "data.title" }];

test("the page receives the same answer, and fetch keeps its shape", async () => {
  const { context, calls } = page({ [API]: { body: '{"data":{"title":"A"}}' } });
  assert.equal(context.fetch.name, "fetch");
  assert.equal(context.fetch.length, 1);
  const init = { method: "POST" };
  const response = await context.fetch(API, init);
  assert.deepEqual(await response.json(), { data: { title: "A" } });
  assert.equal(calls[0].init, init);
  assert.equal(calls[0].self, context);
});

test("an answer that came before anyone asked is still read", async () => {
  const { context, ask } = page({ [API]: { body: '{"data":{"title":"Early"}}' } });
  await context.fetch(API);
  await settle();
  assert.deepEqual(ask("Demo", TITLE), { title: "Early" });
});

test("only the declared address and path are kept, each value small", async () => {
  const other = "https://api.example.com/v2/other";
  const big = "https://api.example.com/v1/player/big";
  const text = "https://api.example.com/v1/player/text";
  const { context, ask } = page({
    [API]: { body: '{"data":{"title":"A","secret":"no"}}' },
    [other]: { body: '{"data":{"title":"Other"}}' },
    [big]: { body: JSON.stringify({ data: { title: "x".repeat(5000) } }) },
    [text]: { body: "not json", type: "text/plain" },
  });
  assert.deepEqual(ask("Demo", TITLE), { title: null });
  await context.fetch(API);
  await context.fetch(other);
  await settle();
  assert.deepEqual(ask("Demo", TITLE), { title: "A" });
  await context.fetch(text);
  await settle();
  assert.deepEqual(ask("Demo", TITLE), { title: "A" }, "an answer that is not JSON leaves the value alone");
  await context.fetch(big);
  await settle();
  assert.deepEqual(ask("Demo", TITLE), { title: null }, "a value too large is not kept");
});

test("answers stop waiting once the page script asked, and after 30 seconds", async () => {
  const late = "https://cdn.example.com/late.json";
  const { context, ask, timers } = page({ [late]: { body: '{"v":1}' } });
  ask("Demo", TITLE);
  await new Promise((resolve) => queueMicrotask(resolve));
  await context.fetch(late);
  await settle();
  assert.deepEqual(ask("Other", [{ name: "v", url: "*://cdn.example.com/*", path: "v" }]), { v: null });

  // Nobody asked: the waiting ends with the timer.
  const second = page({ [late]: { body: '{"v":1}' } });
  for (const timer of second.timers) timer();
  await second.context.fetch(late);
  await settle();
  assert.deepEqual(second.ask("Other", [{ name: "v", url: "*://cdn.example.com/*", path: "v" }]), { v: null });
  assert.equal(timers.length, 1);
});

test("an answer over a megabyte is not read to the end", async () => {
  const huge = "https://api.example.com/v1/player/huge";
  const body = `{"data":{"title":"A","pad":"${"x".repeat(1024 * 1024)}"}}`;
  const { context, ask } = page({ [huge]: { body } });
  ask("Demo", TITLE);
  const response = await context.fetch(huge);
  await settle();
  assert.deepEqual(ask("Demo", TITLE), { title: null });
  assert.equal((await response.text()).length, body.length, "the page still reads all of it");
});

test("XMLHttpRequest answers are read, with one listener per request", () => {
  const { context, ask } = page();
  ask("Demo", TITLE);
  const request = new context.XMLHttpRequest();
  assert.equal(request.open("GET", API), "opened");
  request.open("GET", API);
  assert.equal(request.opened, 2);
  assert.equal(request.listeners.length, 1);
  request.responseType = "";
  request.responseText = '{"data":{"title":"From XHR"}}';
  request.load();
  assert.deepEqual(ask("Demo", TITLE), { title: "From XHR" });

  const json = new context.XMLHttpRequest();
  json.open("GET", API);
  json.responseType = "json";
  json.response = { data: { title: "Parsed" } };
  json.load();
  assert.deepEqual(ask("Demo", TITLE), { title: "Parsed" });
});

test("two presences on one page keep their own values", async () => {
  const { context, ask } = page({ [API]: { body: '{"data":{"title":"A","n":2}}' } });
  ask("One", TITLE);
  ask("Two", [{ name: "n", url: "*://api.example.com/*", path: "data.n" }]);
  await context.fetch(API);
  await settle();
  assert.deepEqual(ask("One", TITLE), { title: "A" });
  assert.deepEqual(ask("Two", [{ name: "n", url: "*://api.example.com/*", path: "data.n" }]), { n: 2 });
});

test("an address that starts with / is read only on the page's own site", async () => {
  const own = "https://www.example.com/api/now?x=1";
  const elsewhere = "https://api.example.com/api/now?x=1";
  const { context, ask } = page({
    [own]: { body: '{"item":{"name":"Mine"}}' },
    [elsewhere]: { body: '{"item":{"name":"Other"}}' },
  });
  const entries = [{ name: "name", url: "/api/now?*", path: "item.name" }];
  ask("Server", entries);
  await context.fetch(elsewhere);
  await settle();
  assert.deepEqual(ask("Server", entries), { name: null });
  await context.fetch(own);
  await settle();
  assert.deepEqual(ask("Server", entries), { name: "Mine" });
});
