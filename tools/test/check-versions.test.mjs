import assert from "node:assert/strict";
import { test } from "node:test";
import { changedPresences, isHigher, review } from "../check-versions.mjs";

const manifest = (version) => JSON.stringify({ version });

test("a version counts as higher by number, not by text", () => {
  assert.equal(isHigher("2.1.10", "2.1.9"), true);
  assert.equal(isHigher("2.1.1", "2.1.1"), false);
  assert.equal(isHigher("2.0.9", "2.1.0"), false);
  assert.equal(isHigher("2.1", "2.0.0"), false);
});

test("changed paths name each presence folder once", () => {
  const paths = [
    "presences/youtube/main.js",
    "presences/youtube/manifest.json",
    "presences/kick/icon.png",
    "presences/README.md",
  ];
  assert.deepEqual(changedPresences(paths), ["kick", "youtube"]);
});

test("a changed presence must raise its version", () => {
  const problems = review(["youtube"], { youtube: manifest("2.1.0") }, { youtube: manifest("2.1.0") });
  assert.equal(problems.length, 1);
  assert.match(problems[0], /youtube/);
});

test("a raised version, a new presence and a removed one pass", () => {
  const before = { youtube: manifest("2.1.0"), old: manifest("1.0.0"), fresh: null };
  const after = { youtube: manifest("2.1.1"), old: null, fresh: manifest("1.0.0") };
  assert.deepEqual(review(["fresh", "old", "youtube"], before, after), []);
});
