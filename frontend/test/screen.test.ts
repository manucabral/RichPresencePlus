import assert from "node:assert/strict";
import { test } from "node:test";
import { createScreen } from "../src/shared/screen.ts";

test("a window that comes back from minimized or the tray is noticed once", () => {
  const screen = createScreen();
  assert.equal(screen.on, true);
  assert.equal(screen.set(true), false);
  assert.equal(screen.set(false), false);
  assert.equal(screen.on, false);
  assert.equal(screen.set(false), false);
  assert.equal(screen.set(true), true);
  assert.equal(screen.on, true);
  assert.equal(screen.set(true), false);
});
