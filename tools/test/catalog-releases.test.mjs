import assert from "node:assert/strict";
import { test } from "node:test";
import { KEEP, prunePlan, releaseFor, releasesOf, tally } from "../catalog-releases.mjs";

const asset = (id, name, created, count = 0) => ({ id, name, created_at: created, download_count: count });

test("the release of a presence matches the application's", () => {
  // Same values as Channel::release in crates/rpp-catalog (first hex digit of SHA-256).
  assert.equal(releaseFor("stable", "youtube"), "catalog-files-2");
  assert.equal(releaseFor("stable", "netflix"), "catalog-files-1");
  assert.equal(releaseFor("experimental", "sa-mp"), "catalog-files-experimental-2");
  assert.equal(releasesOf("stable").length, 16);
  assert.ok(releasesOf("stable").includes(releaseFor("stable", "anything")));
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
