import assert from "node:assert/strict";
import { test } from "node:test";
import { matchPath, placeOf } from "../src/shared/route.ts";

test("a hash becomes a path and its options", () => {
  assert.deepEqual(placeOf(""), { pathname: "/", search: "" });
  assert.deepEqual(placeOf("#/"), { pathname: "/", search: "" });
  assert.deepEqual(placeOf("#/catalog"), { pathname: "/catalog", search: "" });
  assert.deepEqual(placeOf("#/developer?presence=A%20B"), { pathname: "/developer", search: "?presence=A%20B" });
});

test("a pattern matches whole paths and hands over its named parts", () => {
  assert.deepEqual(matchPath("/", "/"), {});
  assert.deepEqual(matchPath("/presences", "/presences"), {});
  assert.deepEqual(matchPath("/presences/:name", "/presences/Ready%20Or%20Not"), { name: "Ready Or Not" });
  assert.equal(matchPath("/presences", "/presences/x"), null);
  assert.equal(matchPath("/presences/:name", "/presences"), null);
  assert.equal(matchPath("/", "/missing"), null);
});

test("a broken escape stays as typed", () => {
  assert.deepEqual(matchPath("/presences/:name", "/presences/100%"), { name: "100%" });
});
