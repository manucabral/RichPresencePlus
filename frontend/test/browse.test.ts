import assert from "node:assert/strict";
import test from "node:test";
import {
  NO_FILTER,
  PAGE_SIZE,
  PAGE_SIZES,
  browse,
  categoryOf,
  countByCategory,
  pageOf,
  paginate,
  rangeOf,
  readPageSize,
} from "../src/shared/browse.ts";
import { catalogPath, inSection, presencePath } from "../src/shared/navigation.ts";
import { catalogError, installsText, sizeText, stateKey, toAdd } from "../src/features/catalog/catalog.ts";
import type { CatalogItem } from "../src/shared/types.ts";

const item = (name: string, kind: string | null, tags: string[] = []) => ({
  name,
  description: null,
  kind,
  author: null,
  tags,
});

const items = [
  item("YouTube", "video"),
  item("Editor", "other", ["code"]),
  item("Roblox", "game"),
  item("Spotify", "music"),
  item("Forum", "other"),
  item("Notepad", null),
];

test("a category says what someone is doing", () => {
  assert.deepEqual(items.map(categoryOf), ["watch", "create", "play", "listen", "more", "more"]);
  assert.equal(categoryOf(item("X", "other", ["Design"])), "create");
});

test("each category is counted", () => {
  assert.deepEqual(countByCategory(items), { watch: 1, listen: 1, play: 1, create: 1, more: 2 });
});

test("favorites come first and can be the only ones", () => {
  const favorites = new Set(["Spotify", "Roblox"]);
  const all = browse(items, NO_FILTER, favorites).map((entry) => entry.name);
  assert.deepEqual(all.slice(0, 2), ["Roblox", "Spotify"]);
  assert.deepEqual(all.slice(2), ["Editor", "Forum", "Notepad", "YouTube"]);
  const only = browse(items, { ...NO_FILTER, favoritesOnly: true }, favorites).map((entry) => entry.name);
  assert.deepEqual(only, ["Roblox", "Spotify"]);
});

test("category and text narrow together", () => {
  const videos = browse(items, { ...NO_FILTER, category: "watch" }, new Set()).map((entry) => entry.name);
  assert.deepEqual(videos, ["YouTube"]);
  const text = browse(items, { ...NO_FILTER, text: "rob" }, new Set()).map((entry) => entry.name);
  assert.deepEqual(text, ["Roblox"]);
  assert.equal(items[0].name, "YouTube", "the input is not reordered");
});

test("a remembered page size must be one on offer", () => {
  for (const size of PAGE_SIZES) assert.equal(readPageSize(String(size)), size);
  for (const stored of [null, "", "0", "-5", "7", "1e9", "ten"]) {
    assert.equal(readPageSize(stored), PAGE_SIZE, String(stored));
  }
});

test("the range counts from 1 and stops at the end of the list", () => {
  assert.deepEqual(rangeOf(23, 0, 10), { first: 1, last: 10 });
  assert.deepEqual(rangeOf(23, 2, 10), { first: 21, last: 23 });
  assert.deepEqual(rangeOf(7, 0, 10), { first: 1, last: 7 });
  assert.deepEqual(rangeOf(0, 0, 10), { first: 0, last: 0 });
});

test("a new size keeps the first item on the page shown", () => {
  // Item 20 opens page 2 of 10 and falls on page 4 of 5 and page 0 of 25.
  assert.equal(pageOf(2 * 10, 5), 4);
  assert.equal(pageOf(2 * 10, 25), 0);
});

test("pages are clamped to the list", () => {
  const list = Array.from({ length: 23 }, (_, n) => n);
  assert.deepEqual(paginate(list, 0, 10), { items: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], page: 0, pages: 3 });
  assert.deepEqual(paginate(list, 2, 10).items, [20, 21, 22]);
  // A page past the end, after a search shrank the list, shows the last one.
  assert.equal(paginate(list, 9, 10).page, 2);
  assert.equal(paginate(list, -3, 10).page, 0);
  assert.deepEqual(paginate([], 4, 10), { items: [], page: 0, pages: 1 });
  assert.deepEqual([0, 9, 10, 25, -1].map((index) => pageOf(index, 10)), [0, 0, 1, 2, 0]);
});

test("detail pages keep a name in one encoded segment", () => {
  assert.equal(presencePath("YouTube Music"), "/presences/YouTube%20Music");
  assert.equal(catalogPath("a/b?c#d"), "/catalog/a%2Fb%3Fc%23d");
});

test("a detail page lights its section in the sidebar", () => {
  assert.ok(inSection("/presences/YouTube", "/presences"));
  assert.ok(inSection("/presences", "/presences"));
  assert.ok(!inSection("/presencesX", "/presences"));
  assert.ok(inSection("/", "/"));
  assert.ok(!inSection("/presences", "/"));
  // Adding presences is a tab of Presences, so the sidebar keeps it lit.
  assert.ok(inSection("/catalog", "/presences"));
  assert.ok(inSection("/catalog/YouTube", "/presences"));
  assert.ok(!inSection("/catalogX", "/presences"));
  assert.ok(!inSection("/catalog", "/privacy"));
});

test("the Add tab lists what is not installed and what needs a step", () => {
  const entry = (name: string, state: CatalogItem["state"]) => ({ name, state }) as CatalogItem;
  const items = [entry("A", "available"), entry("B", "installed"), entry("C", "update"), entry("D", "modified")];
  assert.deepEqual(
    toAdd(items).map((item) => item.name),
    ["A", "C", "D"],
  );
});

test("catalog sizes and states read plainly", () => {
  assert.equal(sizeText(100), "1 KB");
  assert.equal(sizeText(12 * 1024), "12 KB");
  assert.equal(sizeText(3 * 1024 * 1024 + 300_000), "3.3 MB");
  assert.equal(stateKey("available"), null);
  assert.equal(stateKey("update"), "catalog.updateAvailable");
});

test("GitHub's limit on a network reads in the interface's language", () => {
  const t = (key: string) => `[${key}]`;
  const limited = "cannot install: GitHub is limiting downloads from this network for a while; try again later";
  assert.equal(catalogError(new Error(limited), t), "[catalog.rateLimited]");
  assert.equal(catalogError("the catalog signature is missing", t), "the catalog signature is missing");
});

test("installs read as a count in the language in use, or not at all", () => {
  const tn = (key: string, count: number, values?: Record<string, string | number>) =>
    `${key}:${count === 1 ? "one" : "other"}:${values?.n}`;
  const installs = { YouTube: 12345, Roblox: 1, Zero: 0, Odd: 1.5, Text: "9" as unknown as number };
  assert.equal(installsText(installs, "YouTube", "en", tn), "catalog.installs:other:12,345");
  assert.equal(installsText(installs, "YouTube", "es", tn), "catalog.installs:other:12.345");
  assert.equal(installsText(installs, "Roblox", "en", tn), "catalog.installs:one:1");
  for (const name of ["Zero", "Odd", "Text", "Missing", "constructor", "toString"]) {
    assert.equal(installsText(installs, name, "en", tn), null, name);
  }
  // The counts did not arrive, or failed.
  assert.equal(installsText(null, "YouTube", "en", tn), null);
  assert.equal(installsText(undefined, "YouTube", "en", tn), null);
});
