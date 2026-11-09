import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const read = (path) => readFileSync(new URL(`../../${path}`, import.meta.url), "utf8");

test("the updater signature names the version the app requires", () => {
  const config = JSON.parse(read("src-tauri/tauri.conf.json"));
  const workflow = read(".github/workflows/release.yml");
  const signing = workflow.split("\n").filter((line) => line.includes("cargo tauri signer sign"));
  assert.equal(signing.length, 1);
  // With requireSignedVersion on, a signature without the version is refused
  // by every installed copy (seen with 0.2.0 refusing 0.2.1).
  if (config.plugins.updater.requireSignedVersion) assert.match(signing[0], /--app-version "\$\{GITHUB_REF_NAME#v\}"/);
});
