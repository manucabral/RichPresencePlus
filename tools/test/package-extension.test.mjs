import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { missingReferences, packagedFiles, readText, storeManifest } from "../package-extension.mjs";

const extension = fileURLToPath(new URL("../../extension/", import.meta.url));

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    if (name === "node_modules" || name === "test" || name.startsWith(".")) return [];
    const full = join(dir, name);
    return statSync(full).isDirectory() ? walk(full) : [relative(extension, full).replaceAll("\\", "/")];
  });
}

test("the stores get the manifest without its key", () => {
  const manifest = JSON.parse(readFileSync(join(extension, "manifest.json"), "utf8"));
  assert.ok("key" in manifest);
  const store = storeManifest(manifest);
  assert.ok(!("key" in store));
  assert.deepEqual({ ...store, key: manifest.key }, manifest);
});

test("only what the extension runs is packaged", () => {
  const files = packagedFiles([
    "manifest.json",
    "popup.html",
    "background.js",
    "status.js",
    "status.test.js",
    "package.json",
    "pnpm-lock.yaml",
    "README.md",
    "_locales/en/messages.json",
    "_locales/es/messages.json",
    "_locales/en/notes.txt",
    "icons/icon-16.png",
    "icons/icon-128.png",
    "icons/source.svg",
  ]);
  assert.deepEqual(files, [
    "_locales/en/messages.json",
    "_locales/es/messages.json",
    "background.js",
    "icons/icon-128.png",
    "icons/icon-16.png",
    "manifest.json",
    "popup.html",
    "status.js",
  ]);
});

test("every file the real extension refers to is packaged", () => {
  const files = packagedFiles(walk(extension));
  const sources = Object.fromEntries(files.map((file) => [file, readText(join(extension, file))]));
  assert.deepEqual(missingReferences(sources), []);
  // And a missing one is caught.
  const broken = { ...sources };
  delete broken["status.js"];
  assert.ok(missingReferences(broken).includes("status.js"));
  delete broken["icons/icon-48.png"];
  assert.ok(missingReferences(broken).includes("icons/icon-48.png"), "a missing icon is caught too");
});
