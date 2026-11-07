import assert from "node:assert/strict";
import { test } from "node:test";
import { displayName } from "../src/features/presences/program.ts";

test("program names read like names", () => {
  assert.equal(displayName("code.exe"), "Code");
  assert.equal(displayName("my-editor.exe"), "My Editor");
  assert.equal(displayName("some_game.v2.EXE"), "Some Game V2");
  assert.equal(displayName(""), "");
});
