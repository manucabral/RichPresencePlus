import assert from "node:assert/strict";
import { test } from "node:test";
import { detectBrowser } from "../browser.js";

const CHROME_UA =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.6723.117 Safari/537.36";

test("Yandex Browser is not taken for Chrome", () => {
  const userAgent = `${CHROME_UA.replace(" Safari", " YaBrowser/24.12.1.712 Yowser/2.5 Safari")}`;
  assert.deepEqual(detectBrowser({ userAgent }), { name: "yandex", version: "24.12.1.712" });
  // Recent versions also list themselves among the brands.
  const brands = [
    { brand: "Chromium", version: "130" },
    { brand: "YaBrowser", version: "24" },
    { brand: "Yandex", version: "24" },
  ];
  assert.equal(detectBrowser({ userAgent: CHROME_UA, brands }).name, "yandex");
});

test("each Chromium browser is found by its own token", () => {
  const cases = [
    [`${CHROME_UA} Edg/130.0.2849.80`, "edge", "130.0.2849.80"],
    [`${CHROME_UA} OPR/115.0.0.0`, "opera", "115.0.0.0"],
    [`${CHROME_UA} Vivaldi/7.0.3495.18`, "vivaldi", "7.0.3495.18"],
    [CHROME_UA, "chrome", "130.0.6723.117"],
  ];
  for (const [userAgent, name, version] of cases) {
    assert.deepEqual(detectBrowser({ userAgent }), { name, version }, name);
  }
});

test("Brave is found even with Chrome's user agent", () => {
  assert.deepEqual(detectBrowser({ userAgent: CHROME_UA, brave: true }), {
    name: "brave",
    version: "130.0.6723.117",
  });
});

test("Firefox is found, and anything else is other", () => {
  const firefox = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0";
  assert.deepEqual(detectBrowser({ userAgent: firefox }), { name: "firefox", version: "132.0" });
  assert.deepEqual(detectBrowser({ userAgent: "curl/8.0" }), { name: "other", version: "" });
  assert.deepEqual(detectBrowser({}), { name: "other", version: "" });
});

test("a token inside another word does not count, and odd versions are dropped", () => {
  // "NotChrome/1" is not Chrome; "XEdg/2" is not Edge.
  assert.equal(detectBrowser({ userAgent: "Mozilla/5.0 NotChrome/1 XEdg/2" }).name, "other");
  const brands = [{ brand: "Opera", version: "1<b>" }];
  assert.deepEqual(detectBrowser({ userAgent: "", brands }), { name: "opera", version: "" });
});
