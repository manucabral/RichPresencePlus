import assert from "node:assert/strict";
import test from "node:test";
import { idleHint, initial, isExcluded, matches, readableOn, shownOptions, siteNames } from "../src/shared/presence.ts";

const item = {
  name: "YouTube",
  description: "Shows the video",
  kind: "video",
  author: "manucabral",
  tags: ["video", "media"],
};

test("text is readable on light and dark backgrounds", () => {
  assert.equal(readableOn("#FFFFFF"), "#000000");
  assert.equal(readableOn("#0A0A0A"), "#ffffff");
  assert.equal(readableOn("#FF0000"), "#ffffff");
  assert.equal(readableOn("red"), "#ffffff");
});

test("the first letter is drawn when there is no logo", () => {
  assert.equal(initial("  nepu"), "N");
  assert.equal(initial(""), "?");
});

test("the search looks at name, description, author and tags", () => {
  assert.ok(matches(item, "", ""));
  assert.ok(matches(item, "youtube", ""));
  assert.ok(matches(item, "VIDEO", ""));
  assert.ok(matches(item, "cabral", ""));
  assert.ok(matches(item, "media", ""));
  assert.ok(matches(item, "manu video", ""), "every word must match, in any field");
  assert.ok(!matches(item, "spotify", ""));
});

test("the kind filter treats unknown kinds as other", () => {
  assert.ok(matches(item, "", "video"));
  assert.ok(!matches(item, "", "game"));
  assert.ok(matches({ ...item, kind: null }, "", "other"));
  assert.ok(matches({ ...item, kind: "manual" }, "", "other"));
  assert.ok(!matches({ ...item, kind: null }, "", "video"));
});

test("options that depend on a switch hide while it is off", () => {
  const options = [
    { key: "players", type: "boolean", value: false },
    { key: "style", type: "choice", value: "a", if: "players" },
    { key: "size", type: "number", value: 1, if: "style" },
    { key: "deep", type: "boolean", value: true, if: "players" },
    { key: "deeper", type: "text", value: "x", if: "deep" },
    { key: "free", type: "text", value: "y" },
  ];
  assert.deepEqual(shownOptions(options).map((o) => o.key), ["players", "free"]);
  options[0].value = true;
  // `size` depends on a choice, which the backend refuses; it stays hidden.
  assert.deepEqual(shownOptions(options).map((o) => o.key), ["players", "style", "deep", "deeper", "free"]);
  const loop = [
    { key: "a", type: "boolean", value: true, if: "b" },
    { key: "b", type: "boolean", value: true, if: "a" },
  ];
  assert.deepEqual(shownOptions(loop), []);
});

test("site patterns read as sites, each once", () => {
  assert.deepEqual(siteNames(["*://www.youtube.com/*"]), ["www.youtube.com"]);
  assert.deepEqual(siteNames(["https://a.com/x/*", "*://a.com/*"]), ["a.com"]);
  assert.deepEqual(siteNames(["*://*.b.com/*"]), ["b.com"]);
  assert.deepEqual(siteNames(["*://*/*"]), ["*"]);
  assert.deepEqual(siteNames([]), []);
});

test("the idle hint says what would bring an activity", () => {
  const access = (sites: string[], media = false) =>
    ({ sites, media, http: [], process: false, args: {}, window: false, query: [], files: [] });
  const youtube = { name: "YouTube", enabled: true, access: access(["*://www.youtube.com/*"]) };
  const web = { name: "Web Media", enabled: true, access: access(["*://*/*"]) };
  const desktop = { name: "Desktop Media", enabled: true, access: access([], true) };
  const game = { name: "Roblox", enabled: true, access: access([]) };

  assert.deepEqual(idleHint([]), { kind: "none" });
  assert.deepEqual(idleHint([{ ...youtube, enabled: false }]), { kind: "none" });
  assert.deepEqual(idleHint([web, desktop]), { kind: "play" });
  assert.deepEqual(idleHint([web, youtube]), { kind: "open", names: ["YouTube"] });
  const many = [youtube, game, { ...game, name: "Netflix" }, { ...game, name: "Nepu" }];
  assert.deepEqual(idleHint(many), { kind: "open", names: ["YouTube", "Roblox", "Netflix"] });
});

test("a presence is excluded when no site or program is left to it", () => {
  const access = { sites: ["*://www.youtube.com/*"] };
  const video = { name: "YouTube", access } as never;
  const rules = (excluded_domains: string[], excluded_applications: string[] = []) => ({ excluded_domains, excluded_applications });

  assert.equal(isExcluded(video, rules([])), false);
  assert.equal(isExcluded(video, rules(["www.youtube.com"])), true);
  assert.equal(isExcluded(video, rules(["youtube.com"])), true);
  assert.equal(isExcluded(video, rules(["tube.com"])), false);
  const bare = { name: "Video", access: { sites: ["*://youtube.com/*"] } } as never;
  assert.equal(isExcluded(bare, rules(["www.youtube.com"])), true);
  assert.equal(isExcluded(video, rules([], ["youtube"])), true);
  const two = { name: "Video", access: { sites: ["*://a.com/*", "*://b.com/*"] } } as never;
  assert.equal(isExcluded(two, rules(["a.com"])), false);
  assert.equal(isExcluded({ name: "Web Media", access: { sites: ["*://*/*"] } } as never, rules(["a.com"])), false);
});
