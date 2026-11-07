import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { MAX_TAGS, TAGS } from "../src/shared/tags.ts";

const json = (path: string) => JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));

test("the interface offers the words of the manifest format", () => {
  const schema = json("../../crates/rpp-app/templates/manifest.schema.json");
  assert.deepEqual([...TAGS], schema.properties.tags.items.enum);
  assert.equal(MAX_TAGS, schema.properties.tags.maxItems);
});

test("every tag has a label in every language", () => {
  for (const language of ["en", "es"]) {
    const strings = json(`../src/locales/${language}.json`);
    for (const tag of TAGS) assert.ok(strings[`tag.${tag}`], `${language}: ${tag}`);
  }
});
