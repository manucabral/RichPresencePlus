import assert from "node:assert/strict";
import { test } from "node:test";
import { nextStep, setupSteps } from "../src/features/dashboard/setup.ts";

const empty = { discordRunning: false, extensions: 0, enabled: 7, offering: 0 };

test("the guide starts at Discord and moves on", () => {
  assert.equal(nextStep(setupSteps(empty)), "discord");
  assert.equal(nextStep(setupSteps({ ...empty, discordRunning: true })), "browser");
  assert.equal(nextStep(setupSteps({ ...empty, discordRunning: true, extensions: 1 })), "presence");
});

test("the presence step needs one offering an activity", () => {
  const ready = { discordRunning: true, extensions: 1, enabled: 3, offering: 0 };
  assert.equal(nextStep(setupSteps(ready)), "presence");
  assert.equal(nextStep(setupSteps({ ...ready, offering: 1 })), null);
  assert.equal(nextStep(setupSteps({ ...ready, enabled: 0, offering: 1 })), "presence");
});

test("steps with a page link to it", () => {
  const links = setupSteps(empty).map((step) => step.link);
  assert.deepEqual(links, [null, "/browser", "/presences"]);
});
