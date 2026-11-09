import assert from "node:assert/strict";
import { test } from "node:test";
import { latest } from "../latest-json.mjs";

const base = {
  version: "0.2.0",
  repo: "manucabral/RichPresencePlus",
  installerName: "Rich.Presence.Plus_0.2.0_x64-setup.exe",
  signature: "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZw==\n",
  date: new Date("2026-11-03T12:00:00Z"),
};

test("the document points at the release asset and carries the signature", () => {
  const document = latest({ ...base, notes: "Fixes." });
  assert.equal(document.version, "0.2.0");
  assert.equal(document.pub_date, "2026-11-03T12:00:00.000Z");
  const platform = document.platforms["windows-x86_64"];
  assert.equal(platform.signature, "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZw==");
  assert.equal(
    platform.url,
    "https://github.com/manucabral/RichPresencePlus/releases/download/v0.2.0/Rich.Presence.Plus_0.2.0_x64-setup.exe",
  );
});

test("bad input is refused before anything is written", () => {
  assert.throws(() => latest({ ...base, version: "v0.2" }), /semver/);
  assert.throws(() => latest({ ...base, repo: "a/b/c" }), /owner\/name/);
  assert.throws(() => latest({ ...base, repo: "../x" }), /owner\/name/);
  assert.throws(() => latest({ ...base, installerName: "dir/setup.exe" }), /file name/);
  // GitHub would store this one as "Rich.Presence.Plus..." and the address would 404.
  assert.throws(() => latest({ ...base, installerName: "Rich Presence Plus_0.2.0_x64-setup.exe" }), /file name/);
  assert.throws(() => latest({ ...base, signature: "  " }), /empty/);
});
