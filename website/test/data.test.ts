import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { byCategory, hostOf, inkOn, loadPresences, oneOfEach, readsOf } from "../src/data/presences.ts";
import { installerOf, latestInstaller } from "../src/data/release.ts";

const ROOT = fileURLToPath(new URL("../../presences", import.meta.url));

test("hosts come from match patterns", () => {
  assert.equal(hostOf("*://www.netflix.com/*"), "www.netflix.com");
  assert.equal(hostOf("https://*.example.com/watch*"), "*.example.com");
  assert.equal(hostOf("not a pattern"), null);
});

test("a manifest lists only what it declares", () => {
  assert.deepEqual(readsOf({ name: "Nothing" }), []);
  const reads = readsOf({
    name: "All",
    page: { matches: ["*://a.io/*", "https://a.io/x*"], extract: { title: {}, player: {} } },
    permissions: { process: true, http: ["api.a.io"], args: { "a.exe": ["-h"] }, query: ["samp"] },
  });
  assert.deepEqual(reads, [
    { kind: "page", hosts: ["a.io"], values: ["title", "player"] },
    { kind: "process" },
    { kind: "args", programs: ["a.exe"] },
    { kind: "query", protocols: ["samp"] },
    { kind: "http", hosts: ["api.a.io"] },
  ]);
});

test("the frames of a presence are part of what it reads", () => {
  const reads = readsOf({
    name: "Framed",
    page: {
      matches: ["*://a.io/*"],
      extract: { title: {} },
      frames: { matches: ["*://player.b.io/*"], extract: { player: {} } },
    },
  });
  assert.deepEqual(reads, [{ kind: "page", hosts: ["a.io", "player.b.io"], values: ["title", "player"] }]);
});

test("the repository's presences load with their permissions", () => {
  const presences = loadPresences(ROOT);
  const names = presences.map((presence) => presence.name);
  for (const name of ["YouTube", "YouTube Music", "Roblox", "SA-MP", "Netflix"]) {
    assert.ok(names.includes(name), name);
  }
  const roblox = presences.find((presence) => presence.name === "Roblox");
  assert.equal(roblox?.category, "play");
  assert.deepEqual(roblox?.reads.map((read) => read.kind), ["process", "window", "files", "http"]);
  const youtube = presences.find((presence) => presence.name === "YouTube");
  assert.ok(youtube?.description.es, "the Spanish description");
  assert.deepEqual(youtube?.reads.map((read) => read.kind), ["page"]);
});

test("the short list holds one presence of each category", () => {
  const presences = loadPresences(ROOT);
  const picks = oneOfEach(presences, ["YouTube", "YouTube Music", "Roblox"]);
  assert.deepEqual(picks.map((presence) => presence.name).sort(), ["Creator Tools", "Roblox", "YouTube", "YouTube Music"]);
  assert.equal(new Set(picks.map((presence) => presence.category)).size, picks.length);
  // With no preference it still gives one per category.
  assert.equal(oneOfEach(presences, []).length, byCategory(presences).length);
});

test("label color follows the background", () => {
  assert.equal(inkOn("#000000"), "#f7f8f8");
  assert.equal(inkOn("#ffffff"), "#08090a");
  assert.equal(inkOn("#6441a4"), "#f7f8f8");
  assert.equal(inkOn("#19c283"), "#08090a");
});

test("only an installer counts as a download", () => {
  const url = "https://example.com/file";
  const old = { tag_name: "v0.1.1-rc", assets: [{ name: "Rich.Presence.Plus.zip", browser_download_url: url, size: 1 }] };
  assert.equal(installerOf(old), null);
  assert.equal(installerOf({ assets: [] }), null);
  const size = 12 * 1024 * 1024;
  const fresh = {
    tag_name: "v0.2.0",
    assets: [
      { name: "latest.json", browser_download_url: url, size: 10 },
      { name: "Rich Presence Plus_0.2.0_x64-setup.exe", browser_download_url: url, size },
    ],
  };
  assert.deepEqual(installerOf(fresh), { version: "0.2.0", url, size: 12 });
});

test("a failed request does not stop the build", async () => {
  const refused = (async () => {
    throw new Error("offline");
  }) as typeof fetch;
  assert.equal(await latestInstaller(refused), null);
  const missing = (async () => new Response("", { status: 404 })) as typeof fetch;
  assert.equal(await latestInstaller(missing), null);
});
