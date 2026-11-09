import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { cargoVersion, check } from "../version.mjs";

const cargo = (version) =>
  `[workspace]\nmembers = []\n\n[workspace.package]\nedition = "2024"\nversion = "${version}"\n\n[workspace.dependencies]\nserde = { version = "1" }\n`;
const config = (version) => JSON.stringify({ productName: "Rich Presence Plus", version });

test("the program has one version, and a release tag must name it", () => {
  assert.equal(check({ cargo: cargo("0.2.0"), config: config("0.2.0") }), "0.2.0");
  assert.equal(check({ cargo: cargo("0.2.0"), config: config("0.2.0"), tag: "v0.2.0" }), "0.2.0");
  assert.throws(() => check({ cargo: cargo("0.2.0"), config: config("0.1.0") }), /tauri.conf.json says 0.1.0/);
  assert.throws(() => check({ cargo: cargo("0.2.0"), config: config("0.2.0"), tag: "v0.2.1" }), /not v0.2.0/);
  assert.throws(() => check({ cargo: cargo("0.2.0"), config: config("0.2.0"), tag: "0.2.0" }), /not v0.2.0/);
});

test("only the workspace version counts, not a dependency's", () => {
  assert.equal(cargoVersion(cargo("1.2.3")), "1.2.3");
  assert.throws(() => cargoVersion('[dependencies]\nserde = { version = "1" }\n'), /no \[workspace.package\]/);
});

test("the repository's own files agree", () => {
  const read = (path) => readFileSync(new URL(`../../${path}`, import.meta.url), "utf8");
  check({ cargo: read("Cargo.toml"), config: read("src-tauri/tauri.conf.json") });
});
