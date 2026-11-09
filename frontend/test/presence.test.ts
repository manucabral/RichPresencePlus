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

test("the search also finds a presence by its other names", () => {
  const named = { ...item, other_names: ["YT", "Old Tube"] };
  assert.ok(matches(named, "yt", ""));
  assert.ok(matches(named, "old tube", ""));
  assert.ok(!matches(item, "yt", ""));
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
    { key: "players", value: false },
    { key: "style", value: "a", when: "players" },
    { key: "deep", value: true, when: "players" },
    { key: "deeper", value: "x", when: "deep" },
    { key: "free", value: "y" },
  ];
  assert.deepEqual(shownOptions(options).map((o) => o.key), ["players", "free"]);
  options[0].value = true;
  assert.deepEqual(shownOptions(options).map((o) => o.key), ["players", "style", "deep", "deeper", "free"]);
  const loop = [
    { key: "a", value: true, when: "b" },
    { key: "b", value: true, when: "a" },
  ];
  assert.deepEqual(shownOptions(loop), []);
});

test("options that depend on values show only while every value matches", () => {
  const options: { key: string; value: unknown; when?: string | Record<string, unknown> }[] = [
    { key: "image", value: "logo" },
    { key: "show", value: true },
    { key: "size", value: 3 },
    { key: "caption", value: "", when: { image: ["cover", "poster"], show: true } },
    { key: "zoom", value: 1, when: { size: 3 } },
    { key: "inner", value: "", when: { caption: "" } },
    { key: "lost", value: "", when: { missing: 1 } },
  ];
  const keys = () => shownOptions(options).map((o) => o.key);
  assert.deepEqual(keys(), ["image", "show", "size", "zoom"]);
  options[0].value = "cover";
  assert.deepEqual(keys(), ["image", "show", "size", "caption", "zoom", "inner"]);
  options[1].value = false;
  assert.deepEqual(keys(), ["image", "show", "size", "zoom"], "every value must match");
  options[2].value = 4;
  assert.deepEqual(keys(), ["image", "show", "size"]);
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
