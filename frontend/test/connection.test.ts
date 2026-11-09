import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { browserList, browserMark, discordAdvice, discordText } from "../src/shared/connection.ts";
import { translate } from "../src/shared/i18n/core.ts";
import { shortcutOf } from "../src/features/settings/shortcut.ts";

const table = JSON.parse(readFileSync(join(import.meta.dirname, "../src/locales/en.json"), "utf8"));
const t = (key: string, values?: Record<string, string | number>) => translate(table, key, values);

test("connected browsers are named once, with a count for repeats", () => {
  assert.equal(browserList([], t), "");
  assert.equal(browserList(["chrome"], t), "Chrome");
  assert.equal(browserList(["chrome", "edge", "edge", "firefox"], t), "Chrome, Edge (2), Firefox");
  assert.equal(browserList(["other", "other"], t), "Another browser (2)");
});

test("an unexpected browser name is never shown as it came", () => {
  for (const odd of ["toString", "__proto__", "constructor", "<b>x</b>"]) {
    assert.equal(browserList([odd], t), "Another browser", odd);
  }
});

test("the Discord card names the account once Discord told it", () => {
  assert.equal(discordText(false, "tester", t), "Not running");
  assert.equal(discordText(true, null, t), "Running");
  assert.equal(discordText(true, "tester", t), "Publishing as tester");
  // A name that looks like a placeholder stays as it is.
  assert.equal(discordText(true, "{name}", t), "Publishing as {name}");
});

test("advice appears only while Discord is not running", () => {
  assert.equal(discordAdvice(true, t), null);
  assert.match(discordAdvice(false, t) ?? "", /Discord for desktop/);
});

test("a key press becomes a shortcut in the stored order", () => {
  const press = (code: string, mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) => ({
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    code,
    ...mods,
  });
  assert.equal(shortcutOf(press("KeyP", { altKey: true, ctrlKey: true })), "Ctrl+Alt+P");
  assert.equal(shortcutOf(press("F12", { shiftKey: true, metaKey: true })), "Shift+Super+F12");
  assert.equal(shortcutOf(press("Digit9", { ctrlKey: true })), "Ctrl+9");
  // Modifiers alone, and keys a shortcut cannot use, give nothing yet.
  for (const code of ["ControlLeft", "Space", "F25", "Semicolon", "Numpad1"]) {
    assert.equal(shortcutOf(press(code, { ctrlKey: true })), null, code);
  }
});

test("each browser gets its own letter and tint, never a maker's logo", () => {
  const marks = ["chrome", "edge", "firefox", "opera", "brave", "vivaldi", "yandex"].map(browserMark);
  // Edge and Firefox used to share the M of their makers' names.
  assert.deepEqual(
    marks.map((mark) => mark.letter),
    ["C", "E", "F", "O", "B", "V", "Y"],
  );
  assert.equal(new Set(marks.map((mark) => mark.tint)).size, marks.length);
  assert.deepEqual(browserMark("other"), { letter: "?", tint: "#71717a" });
  assert.equal(browserMark("toString").letter, "?");
});
