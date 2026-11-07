import assert from "node:assert/strict";
import test from "node:test";
import { clock, edited, lines, minutesOf } from "../src/features/privacy/form.ts";

const saved = {
  paused: false,
  privacy: "full",
  excluded_domains: ["example.com"],
  excluded_applications: [],
  switch_delay_seconds: 3,
  idle_minutes: null,
  paused_minutes: 0,
  prefer_recent: false,
  quiet_programs: [],
  quiet_hours: null,
};

test("a text box gives one entry per line, without blanks", () => {
  assert.deepEqual(lines(" a.com \n\n b.com\n"), ["a.com", "b.com"]);
});

test("there is nothing to save until something changes", () => {
  assert.equal(edited(saved, { ...saved }, "example.com", "", ""), false);
  assert.equal(edited(saved, { ...saved }, " example.com\n\n", "  ", ""), false);
  assert.equal(edited(saved, { ...saved, paused: true }, "example.com", "", ""), true);
  assert.equal(edited(saved, { ...saved }, "example.com\nother.com", "", ""), true);
  assert.equal(edited(saved, { ...saved }, "example.com", "game.exe", ""), true);
});

test("minutes after midnight and a time box say the same", () => {
  assert.equal(clock(0), "00:00");
  assert.equal(clock(22 * 60 + 5), "22:05");
  assert.equal(minutesOf("22:05"), 22 * 60 + 5);
  assert.equal(minutesOf(""), null);
});

test("a change in the programs that hide everything is something to save", () => {
  assert.equal(edited(saved, saved, "example.com", "", ""), false);
  assert.equal(edited(saved, saved, "example.com", "", "obs64.exe"), true);
});
