import assert from "node:assert/strict";
import { test } from "node:test";
import { keyName } from "../src/shared/desktop.ts";

const press = (key: string, more: Partial<KeyboardEvent> = {}) =>
  keyName({ key, ctrlKey: false, metaKey: false, shiftKey: false, ...more });

test("a key is written with its modifiers", () => {
  assert.equal(press("F5"), "f5");
  assert.equal(press("r", { ctrlKey: true }), "ctrl+r");
  assert.equal(press("I", { ctrlKey: true, shiftKey: true }), "ctrl+shift+i");
});
