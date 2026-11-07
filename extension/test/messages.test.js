import assert from "node:assert/strict";
import { test } from "node:test";
import { fitsLink, MAX_PAGE_BYTES, mayReceive, pageHost, pickedHost } from "../messages.js";

const self = { id: "abc", base: "chrome-extension://abc/" };
const popup = { id: "abc", url: "chrome-extension://abc/popup.html" };
const page = { id: "abc", url: "https://www.youtube.com/watch", tab: { id: 4, incognito: false } };

test("page scripts may only report pages and ask for rules", () => {
  for (const type of ["rules", "page", "left"]) assert.ok(mayReceive({ type }, page, self), type);
  // A script in a web page cannot pair, pause, or read the status.
  for (const type of ["status", "pair", "refresh", "tab", "pause"]) {
    assert.ok(!mayReceive({ type }, page, self), type);
  }
});

test("the popup may run the popup's actions and nothing a page does", () => {
  for (const type of ["status", "pair", "refresh", "tab", "pause"]) assert.ok(mayReceive({ type }, popup, self), type);
  for (const type of ["page", "left"]) assert.ok(!mayReceive({ type }, popup, self), type);
});

test("other extensions, other origins and odd messages are refused", () => {
  assert.ok(!mayReceive({ type: "status" }, { ...popup, id: "evil" }, self));
  assert.ok(!mayReceive({ type: "page" }, { ...page, id: "evil" }, self));
  assert.ok(!mayReceive({ type: "status" }, { id: "abc", url: "https://abc.example/" }, self));
  assert.ok(!mayReceive({ type: "status" }, { id: "abc" }, self));
  for (const message of [null, "status", {}, { type: 1 }, { type: "eval" }, { type: "__proto__" }]) {
    assert.ok(!mayReceive(message, popup, self), JSON.stringify(message));
  }
  assert.ok(!mayReceive({ type: "status" }, null, self));
});

test("a page report too long for the link does not leave", () => {
  const report = (url) => ({ type: "page", tab: 1, presence: "Video", url, title: "", values: {} });
  assert.equal(fitsLink(report("https://a.com/")), true);
  assert.equal(fitsLink(report(`https://a.com/${"x".repeat(MAX_PAGE_BYTES)}`)), false);
  // The limit counts bytes: this text is short in characters.
  assert.equal(fitsLink(report("€".repeat(MAX_PAGE_BYTES / 2))), false);
  // Sealed, the largest report still fits what the app accepts.
  assert.ok((MAX_PAGE_BYTES * 4) / 3 + 1024 < 256 * 1024);
});

test("choosing a site belongs to the extension's own pages", () => {
  const self = { id: "me", base: "chrome-extension://me/" };
  const message = { type: "open-site", origin: "https://a.com/*", on: true };
  assert.equal(mayReceive(message, { id: "me", url: "chrome-extension://me/popup.html" }, self), true);
  assert.equal(mayReceive(message, { id: "me", tab: { id: 1 }, url: "https://a.com/" }, self), false);
});

test("only the popup starts a pick, and a page can only end one", () => {
  assert.ok(mayReceive({ type: "pick-start" }, popup, self));
  assert.ok(!mayReceive({ type: "pick-start" }, page, self));
  assert.ok(mayReceive({ type: "pick-done" }, page, self));
  assert.ok(!mayReceive({ type: "pick-done" }, popup, self));
});

test("a presence is made for web pages only", () => {
  assert.equal(pageHost("https://WWW.Example.com/a?b"), "www.example.com");
  assert.equal(pageHost("http://127.0.0.1:8080/"), "127.0.0.1");
  for (const url of ["chrome://settings", "file:///c:/a.html", "about:blank", "not an address", undefined]) {
    assert.equal(pageHost(url), null, url);
  }
});

test("a pick ends on the site where it started, or not at all", () => {
  const tab = { url: "https://blog.example.com/post", incognito: false };
  const pick = { name: "Blog", first: "h1", second: null };
  assert.equal(pickedHost("blog.example.com", tab, pick), "blog.example.com");
  assert.equal(pickedHost("blog.example.com", tab, { ...pick, second: "h2" }), "blog.example.com");
  // Nothing was started, or the tab moved to another site meanwhile.
  assert.equal(pickedHost(undefined, tab, pick), null);
  assert.equal(pickedHost("other.example.com", tab, pick), null);
  assert.equal(pickedHost("blog.example.com", { ...tab, incognito: true }, pick), null);
  // A message names no site of its own, and bad values are refused.
  assert.equal(pickedHost("blog.example.com", tab, { ...pick, host: "evil.example" }), "blog.example.com");
  assert.equal(pickedHost("blog.example.com", tab, { ...pick, first: "" }), null);
  assert.equal(pickedHost("blog.example.com", tab, { ...pick, first: "x".repeat(300) }), null);
  assert.equal(pickedHost("blog.example.com", tab, { ...pick, name: 5 }), null);
  assert.equal(pickedHost("blog.example.com", tab, { ...pick, second: {} }), null);
});
