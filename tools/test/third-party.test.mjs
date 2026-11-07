import assert from "node:assert/strict";
import { test } from "node:test";
import { groupByLicense, normalize } from "../third-party.mjs";

test("the same license written in different ways counts as one", () => {
  assert.equal(normalize("MIT/Apache-2.0"), "Apache-2.0 OR MIT");
  assert.equal(normalize("MIT OR Apache-2.0"), "Apache-2.0 OR MIT");
  assert.equal(normalize("Apache-2.0 / MIT"), "Apache-2.0 OR MIT");
  assert.equal(normalize("MIT"), "MIT");
});

test("a compound or a missing license stays visible", () => {
  assert.equal(normalize("(MIT OR Apache-2.0) AND Unicode-3.0"), "(MIT OR Apache-2.0) AND Unicode-3.0");
  assert.equal(normalize(null), "Unknown");
  assert.equal(normalize("  "), "Unknown");
});

test("groups hold each name once, the largest group first", () => {
  const groups = groupByLicense([
    { name: "b", license: "MIT" },
    { name: "a", license: "MIT" },
    { name: "a", license: "MIT" },
    { name: "c", license: "ISC" },
  ]);
  assert.deepEqual(groups, [
    { license: "MIT", names: ["a", "b"] },
    { license: "ISC", names: ["c"] },
  ]);
});
