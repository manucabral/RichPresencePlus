/**
 * The manifest schema that code editors read has to match what the application
 * accepts: every field of the real presences and of the templates must be in it.
 */
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";

const path = (relative: string) => new URL(relative, import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const schema = JSON.parse(readFileSync(path("../../crates/rpp-app/templates/manifest.schema.json"), "utf8"));
const presences = path("../../presences/");
const manifests = [
  ...readdirSync(presences).map((folder) => join(presences, folder, "manifest.json")),
  path("../../crates/rpp-app/templates/web/manifest.json"),
  path("../../crates/rpp-app/templates/desktop/manifest.json"),
].map((file) => JSON.parse(readFileSync(file, "utf8")));

test("the schema knows every field a manifest uses", () => {
  for (const manifest of manifests) {
    for (const key of Object.keys(manifest)) assert.ok(key in schema.properties, `${manifest.name}: ${key}`);
    for (const key of Object.keys(manifest.permissions ?? {})) {
      assert.ok(key in schema.properties.permissions.properties, `${manifest.name}: permissions.${key}`);
    }
    for (const setting of manifest.settings ?? []) {
      for (const key of Object.keys(setting)) assert.ok(key in schema.definitions.setting.properties, `${manifest.name}: ${key}`);
    }
  }
});

test("the schema knows every kind of value a page can give", () => {
  const known = schema.definitions.extractor.oneOf.map((kind: { properties: { from: { const: string } } }) => kind.properties.from.const);
  for (const manifest of manifests) {
    for (const extractor of Object.values<{ from: string }>(manifest.page?.extract ?? {})) {
      assert.ok(known.includes(extractor.from), `${manifest.name}: ${extractor.from}`);
    }
  }
  assert.deepEqual([...known].sort(), ["attr", "json", "list", "media", "meta", "session", "storage", "style", "text", "variable"]);
});

test("no manifest names its schema: the editor's settings connect them", () => {
  for (const manifest of manifests) assert.ok(!("$schema" in manifest), manifest.name);
});
