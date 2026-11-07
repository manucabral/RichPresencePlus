// The color system: one set of tokens, readable pairings, and no color that
// escapes it in the application, the popup or the website.
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../..", import.meta.url));
const read = (path) => readFileSync(join(ROOT, path), "utf8");
const tokens = read("design/tokens.css");

/** The declared value of a token, following references to other tokens. */
function value(name) {
  const found = new RegExp(`--${name}:\\s*([^;]+);`).exec(tokens);
  assert.ok(found, `missing token ${name}`);
  const reference = /^var\(--([a-z0-9-]+)\)$/.exec(found[1].trim());
  return reference ? value(reference[1]) : found[1].trim();
}

function luminance(hex) {
  const channels = [1, 3, 5].map((at) => Number.parseInt(hex.slice(at, at + 2), 16) / 255);
  const [r, g, b] = channels.map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a, b) {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

test("the logo's greens are the steps of the scale", () => {
  const logo = read("frontend/public/logo.svg").toLowerCase();
  assert.ok(logo.includes(value("green-300")));
  assert.ok(logo.includes(value("green-500")));
});

test("every text role is readable on every surface", () => {
  for (const surface of ["surface-page", "surface-panel"]) {
    for (const text of [
      "text-primary",
      "text-secondary",
      "text-muted",
      "text-hint",
      "accent",
      "accent-strong",
      "warning",
      "danger",
    ]) {
      const ratio = contrast(value(text), value(surface));
      assert.ok(ratio >= 4.5, `${text} on ${surface}: ${ratio.toFixed(2)}`);
    }
  }
});

test("text on the brand green reads, and white would not", () => {
  assert.ok(contrast(value("accent-ink"), value("accent")) >= 4.5);
  assert.ok(contrast(value("accent-ink"), value("accent-hover")) >= 4.5);
  assert.ok(contrast("#ffffff", value("accent")) < 4.5);
});

test("the popup keeps the shared values", () => {
  const popup = read("extension/popup.html");
  for (const [local, token] of [
    ["page", "surface-page"],
    ["panel", "surface-panel"],
    ["border", "border"],
    ["text", "text-primary"],
    ["muted", "text-muted"],
    ["hint", "text-hint"],
    ["accent", "accent"],
    ["accent-soft", "accent-strong"],
    ["accent-hover", "accent-hover"],
    ["accent-ink", "accent-ink"],
    ["warning", "warning"],
    ["danger", "danger"],
  ]) {
    const declared = new RegExp(`--${local}:\\s*(#[0-9a-f]{6});`).exec(popup)?.[1];
    assert.equal(declared, value(token), local);
  }
});

function files(dir, extensions) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return files(path, extensions);
    return extensions.some((end) => name.endsWith(end)) ? [path] : [];
  });
}

test("the interface uses roles, not colors of its own", () => {
  const raw = /\b(?:bg|text|border|ring|fill)-(?:red|amber|emerald|green|blue|indigo|yellow)-\d/;
  for (const path of files(join(ROOT, "frontend/src"), [".tsx"])) {
    assert.doesNotMatch(readFileSync(path, "utf8"), raw, path);
  }
  const style = read("frontend/src/index.css");
  assert.doesNotMatch(style, /#[0-9a-f]{3,8}\b/i, "index.css");
});

test("the website takes its colors from the tokens", {
  skip: !existsSync(join(ROOT, "website/src/styles/site.css")),
}, () => {
  for (const path of ["website/src/styles/site.css", "design/accordion.css"]) {
    const style = read(path);
    assert.doesNotMatch(style, /#[0-9a-f]{3,8}\b/i, path);
    assert.doesNotMatch(style, /rgba?\(/i, path);
  }
});
