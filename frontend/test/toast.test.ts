import assert from "node:assert/strict";
import { mock, test } from "node:test";
import { dismiss, toast, watchToasts, type Toast } from "../src/shared/toast.ts";

test("notices leave on their own, by hand, and when too many pile up", () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  let seen: Toast[] = [];
  const stop = watchToasts((toasts) => {
    seen = toasts;
  });

  toast("one", { description: "detail" });
  assert.deepEqual(seen.map((item) => [item.title, item.description]), [["one", "detail"]]);
  mock.timers.tick(4000);
  assert.deepEqual(seen, []);

  toast("a");
  toast("b");
  toast("c");
  toast("d");
  assert.deepEqual(seen.map((item) => item.title), ["b", "c", "d"]);
  dismiss(seen[0].id);
  assert.deepEqual(seen.map((item) => item.title), ["c", "d"]);

  stop();
  toast("unseen");
  assert.deepEqual(seen.map((item) => item.title), ["c", "d"]);
  mock.timers.reset();
});
