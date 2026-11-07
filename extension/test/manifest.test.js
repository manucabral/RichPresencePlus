// The manifest has to load in Chrome, Edge and Firefox without changes.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const manifest = JSON.parse(readFileSync(new URL("../manifest.json", import.meta.url), "utf8"));

test("the background runs as a service worker and as a script", () => {
  // Chrome and Edge use service_worker; Firefox ignores it and uses scripts.
  assert.equal(manifest.background.service_worker, "background.js");
  assert.deepEqual(manifest.background.scripts, ["background.js"]);
});

test("Firefox gets an add-on id and a minimum version", () => {
  assert.match(manifest.browser_specific_settings.gecko.id, /@/);
  assert.ok(Number.parseInt(manifest.browser_specific_settings.gecko.strict_min_version, 10) >= 128);
});

test("the page policy does not upgrade ws:// to wss://", () => {
  // Firefox's default policy for extension pages adds upgrade-insecure-requests,
  // which makes the connection to the local app fail with a TLS error.
  const policy = manifest.content_security_policy.extension_pages;
  assert.ok(!policy.includes("upgrade-insecure-requests"));
  assert.ok(policy.includes("script-src 'self'"));
});

test("the extension may start the desktop app's native host", () => {
  assert.ok(manifest.permissions.includes("nativeMessaging"));
  // The key fixes the development ID the host accepts; it is checked against
  // the allowed list in crates/rpp-bridge/src/native.rs.
  assert.match(manifest.key, /^[A-Za-z0-9+/]+=*$/);
});

test("Firefox is told which data leaves the browser", () => {
  const gecko = manifest.browser_specific_settings.gecko;
  // Built-in consent needs Firefox 140; it is required for new add-ons.
  assert.ok(Number.parseInt(gecko.strict_min_version, 10) >= 140);
  assert.deepEqual(gecko.data_collection_permissions.required.sort(), ["browsingActivity", "websiteContent"]);
});

test("no permission reaches every site without asking", () => {
  // Sites are optional and requested per presence; activeTab needs a click.
  assert.ok(!("host_permissions" in manifest));
  assert.deepEqual(manifest.optional_host_permissions, ["*://*/*"]);
  for (const risky of ["tabs", "history", "cookies", "webRequest", "<all_urls>", "debugger"]) {
    assert.ok(!manifest.permissions.includes(risky), risky);
  }
  assert.ok(!("externally_connectable" in manifest), "web pages must not message the extension");
});

/** Width and height from a PNG's header (the IHDR chunk comes first). */
function pngSize(path) {
  const bytes = readFileSync(new URL(`../${path}`, import.meta.url));
  assert.equal(bytes.toString("latin1", 1, 4), "PNG", path);
  return [bytes.readUInt32BE(16), bytes.readUInt32BE(20)];
}

test("the stores and the toolbar get icons at their real sizes", () => {
  // Chrome Web Store, Edge and AMO ask for 128; the browsers use 16, 32 and 48.
  assert.deepEqual(
    Object.keys(manifest.icons).sort((a, b) => a - b),
    ["16", "32", "48", "128"],
  );
  const all = { ...manifest.icons, ...manifest.action.default_icon };
  for (const [size, path] of Object.entries(all)) {
    const n = Number(size);
    assert.deepEqual(pngSize(path), [n, n], path);
  }
});
