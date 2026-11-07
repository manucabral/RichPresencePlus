import assert from "node:assert/strict";
import test from "node:test";
import { dayKey, hoursAndMinutes, lastWeek, namesOfTheWeek } from "../src/features/week/week.ts";

const today = new Date(2026, 10, 5, 13, 30);

test("a date is keyed the way the host keys it", () => {
  assert.equal(dayKey(new Date(2026, 0, 3)), "2026-01-03");
});

test("the week is the seven days that end today, with the empty ones", () => {
  const week = lastWeek(today, { "2026-11-05": { A: 60, B: 30 }, "2026-10-30": { A: 5 }, "2026-10-29": { A: 999 } });
  assert.equal(week.length, 7);
  assert.equal(week[0].date, "2026-10-30");
  assert.equal(week[0].seconds, 5);
  assert.equal(week[6].date, "2026-11-05");
  assert.equal(week[6].seconds, 90);
  assert.equal(week[3].seconds, 0);
});

test("the week crosses the end of a month and of a year", () => {
  assert.equal(lastWeek(new Date(2027, 0, 2), {})[0].date, "2026-12-27");
});

test("presences add up over the week, most time first", () => {
  const names = namesOfTheWeek(today, {
    "2026-11-05": { A: 60, B: 300 },
    "2026-11-04": { A: 60 },
    "2026-10-01": { C: 9999 },
  });
  assert.deepEqual(names, [
    { name: "B", seconds: 300 },
    { name: "A", seconds: 120 },
  ]);
});

test("a time splits into hours and minutes", () => {
  assert.deepEqual(hoursAndMinutes(5400 + 59), { hours: 1, minutes: 30 });
  assert.deepEqual(hoursAndMinutes(59), { hours: 0, minutes: 0 });
});
