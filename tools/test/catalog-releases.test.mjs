import assert from "node:assert/strict";
import { test } from "node:test";
import { KEEP, prunePlan, releaseFor, releasesOf, tally } from "../catalog-releases.mjs";

const asset = (id, name, created, count = 0) => ({ id, name, created_at: created, download_count: count });

test("each channel keeps its bundles in one release, as the application asks", () => {
  // Same tags as Channel::release in crates/rpp-catalog.
  assert.equal(releaseFor("stable"), "catalog-files");
  assert.equal(releaseFor("experimental"), "catalog-files-experimental");
  assert.deepEqual(releasesOf("stable"), ["catalog-files"]);
  assert.deepEqual(releasesOf("experimental"), ["catalog-files-experimental"]);
});

test("each presence keeps the bundle in use and the one before", () => {
  const assets = [
    asset(1, "youtube_1.0.0_aaaaaaaaaaaa.json", "2026-01-01T00:00:00Z"),
    asset(2, "youtube_1.1.0_bbbbbbbbbbbb.json", "2026-02-01T00:00:00Z"),
    asset(3, "youtube_1.2.0_cccccccccccc.json", "2026-03-01T00:00:00Z"),
    asset(4, "netflix_1.0.0_dddddddddddd.json", "2026-01-01T00:00:00Z"),
    asset(5, "notes.txt", "2020-01-01T00:00:00Z"),
  ];
  const current = new Set(["youtube_1.2.0_cccccccccccc.json", "netflix_1.0.0_dddddddddddd.json"]);
  assert.equal(KEEP, 2);
  assert.deepEqual(
    prunePlan(assets, current).map((a) => a.id),
    [1],
  );
  // A rollback to an older index keeps what it names, even if it is not the newest.
  const rollback = new Set(["youtube_1.0.0_aaaaaaaaaaaa.json"]);
  assert.deepEqual(
    prunePlan(assets, rollback).map((a) => a.id),
    [2],
  );
  // A presence that left the catalog keeps its two newest; other files are never deleted.
  assert.deepEqual(
    prunePlan(assets, new Set()).map((a) => a.id),
    [1],
  );
});

test("installs add live downloads and deleted bundles once", () => {
  const live = [
    asset(3, "youtube_1.2.0_cccccccccccc.json", "x", 10),
    asset(4, "netflix_1.0.0_dddddddddddd.json", "x", 4),
    asset(9, "notes.txt", "x", 99),
  ];
  const retired = [
    { id: "1", folder: "youtube", count: 50 },
    // Written to the ledger, but the delete failed: it is still live, counted once.
    { id: "4", folder: "netflix", count: 3 },
    { id: "7", folder: "../evil", count: 1000 },
    { id: "8", folder: "roblox", count: -5 },
  ];
  assert.deepEqual(tally(live, retired), [
    { folder: "youtube", installs: 60 },
    { folder: "netflix", installs: 4 },
    { folder: "roblox", installs: 0 },
  ]);
});

test("the cleanup loads from a copy of the tools without the schema", async () => {
  // The workflow copies tools/ out of the checkout before switching branches.
  const { cpSync, mkdtempSync, rmSync } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");
  const { pathToFileURL } = await import("node:url");
  const home = mkdtempSync(join(tmpdir(), "rpp-tools-"));
  try {
    cpSync(new URL("..", import.meta.url), join(home, "tools"), {
      recursive: true,
      filter: (path) => !path.includes("node_modules"),
    });
    const copy = await import(pathToFileURL(join(home, "tools", "catalog-releases.mjs")).href);
    assert.equal(copy.releaseFor("stable"), "catalog-files");
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
});
