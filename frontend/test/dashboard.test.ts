import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { deliveryText } from "../src/features/dashboard/reason.ts";
import { MANUAL_SOURCE, optionsLink, ordered, priorityBars, toneOf } from "../src/features/dashboard/tone.ts";
import { browserName, duration } from "../src/shared/connection.ts";
import { translate } from "../src/shared/i18n/core.ts";

const table = JSON.parse(readFileSync(join(import.meta.dirname, "../src/locales/en.json"), "utf8"));
const t = (key: string, values?: Record<string, string | number>) => translate(table, key, values);

test("each policy reason gets its icon, and the selected row shows", () => {
  assert.equal(toneOf("anything", true), "showing");
  assert.equal(toneOf("Waiting before switching", false), "waiting");
  assert.equal(toneOf("Media is paused", false), "paused");
  assert.equal(toneOf("Presence stopped reporting", false), "stopped");
  assert.equal(toneOf("Another activity has a higher priority; on a tie the current one stays", false), "standby");
  assert.equal(toneOf("Hidden while the computer is idle", false), "hidden");
  assert.equal(toneOf("toString", false), "hidden");
  assert.equal(toneOf("Something new", false), "hidden");
});

test("the reasons the icons know are the ones the policy writes", () => {
  const policy = readFileSync(join(import.meta.dirname, "../../crates/rpp-core/src/policy.rs"), "utf8");
  const tone = readFileSync(join(import.meta.dirname, "../src/features/dashboard/tone.ts"), "utf8");
  const known = [...tone.matchAll(/^\s+"([^"]+)": "[a-z]+",$/gm)].map((match) => match[1]);
  assert.ok(known.length >= 9);
  for (const reason of known) assert.ok(policy.includes(`"${reason}"`), `policy.rs no longer says: ${reason}`);
});

test("priority becomes one to three bars", () => {
  assert.deepEqual([0, 33, 34, 66, 67, 100].map(priorityBars), [1, 1, 2, 2, 3, 3]);
});

test("the row on display comes first, then priority, then name", () => {
  const rows = [
    { label: "b", priority: 40, selected: false },
    { label: "a", priority: 40, selected: false },
    { label: "z", priority: 10, selected: true },
    { label: "c", priority: 80, selected: false },
  ];
  assert.deepEqual(ordered(rows).map((row) => row.label), ["z", "c", "a", "b"]);
  assert.equal(rows[0].label, "b", "the input is not reordered");
});

test("durations read in minutes or hours", () => {
  assert.equal(duration(-5, t), "less than a minute");
  assert.equal(duration(59, t), "less than a minute");
  assert.equal(duration(12 * 60 + 30, t), "12 min");
  assert.equal(duration(65 * 60, t), "1 h 05 min");
});

test("browsers get their makers' names", () => {
  assert.equal(browserName("yandex", t), "Yandex Browser");
  assert.equal(browserName("edge", t), "Microsoft Edge");
  assert.equal(browserName("__proto__", t), "Another browser");
});

test("options open the presence's card, or the custom status page", () => {
  const manual = readFileSync(join(import.meta.dirname, "../../crates/rpp-app/src/manual.rs"), "utf8");
  assert.ok(manual.includes(`pub const SOURCE: &str = "${MANUAL_SOURCE}";`));
  assert.equal(optionsLink(MANUAL_SOURCE), "/manual");
  assert.equal(optionsLink("YouTube Music"), "/presences/YouTube%20Music");
  // A name can never leave the presences section or add a query.
  assert.equal(optionsLink("../settings?x=1#y"), "/presences/..%2Fsettings%3Fx%3D1%23y");
});

test("a program in Discord's place is named", () => {
  assert.equal(deliveryText("Fake.exe holds the connection to Discord", t), "Fake.exe holds the connection to Discord. Close it to publish.");
  assert.ok(deliveryText("Discord is not running: gone", t).includes("Discord is not running: gone"));
});
