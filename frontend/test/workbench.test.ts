import assert from "node:assert/strict";
import { test } from "node:test";
import { allPassed, firstTab, lineSummary, sizeText } from "../src/features/developer/workbench.ts";

test("sizes read in the unit that fits", () => {
  assert.equal(sizeText(512), "512 B");
  assert.equal(sizeText(2048), "2 KB");
  assert.equal(sizeText(3 * 1024 * 1024), "3.0 MB");
});

test("the line summary leaves out what did not move", () => {
  assert.equal(lineSummary({ added: 3, removed: 1 }), "+3 −1");
  assert.equal(lineSummary({ added: 0, removed: 2 }), "−2");
  assert.equal(lineSummary({ added: 0, removed: 0 }), "");
});

test("a visit starts on creating until there is something to work on", () => {
  assert.equal(firstTab(false, false), "create");
  assert.equal(firstTab(true, false), "code");
  assert.equal(firstTab(false, true), "test");
  assert.equal(firstTab(true, true), "test");
});

test("a presence is ready when every rule passed, and not before they load", () => {
  assert.equal(allPassed([]), false);
  assert.equal(allPassed([{ passed: true }, { passed: false }]), false);
  assert.equal(allPassed([{ passed: true }, { passed: true }]), true);
});
