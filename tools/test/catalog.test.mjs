import assert from "node:assert/strict";
import { generateKeyPairSync } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { BUNDLE_NAME, buildPayload, bundleOf, check, envelope, publicKeyOf } from "../catalog.mjs";

const COMMIT = "0123456789abcdef0123456789abcdef01234567";
const key = () => generateKeyPairSync("ed25519").privateKey.export({ type: "pkcs8", format: "pem" });

/** A presences folder with the given presences: `{ folder: { version, files } }`. */
function presences(spec) {
  const root = mkdtempSync(join(tmpdir(), "rpp-catalog-"));
  for (const [folder, { version = "1.0.0", files = {} }] of Object.entries(spec)) {
    mkdirSync(join(root, folder), { recursive: true });
    const manifest = {
      name: folder,
      version,
      client_id: "12345678901234567",
      entry: "main.js",
      kind: "video",
      description: "Shows things.",
    };
    writeFileSync(join(root, folder, "manifest.json"), JSON.stringify(manifest));
    writeFileSync(join(root, folder, "main.js"), "function read(){return null}");
    for (const [path, text] of Object.entries(files)) {
      mkdirSync(join(root, folder, path, ".."), { recursive: true });
      writeFileSync(join(root, folder, path), text);
    }
  }
  return root;
}

test("the payload lists every file with its digest, folders sorted", () => {
  const root = presences({ beta: { files: { "locales/en.json": "{}" } }, alpha: {} });
  const payload = JSON.parse(buildPayload({ root, commit: COMMIT, generatedAt: 1790000000 }));
  assert.deepEqual(
    payload.presences.map((p) => p.folder),
    ["alpha", "beta"],
  );
  assert.deepEqual(Object.keys(payload.presences[1].files), ["locales/en.json", "main.js", "manifest.json"]);
  assert.equal(payload.presences[0].files["main.js"].size, "function read(){return null}".length);
  assert.match(payload.presences[0].files["main.js"].sha256, /^[0-9a-f]{64}$/);
  assert.equal(payload.schema_version, 1);
  assert.equal(payload.generated_at, 1790000000);
});

test("the index carries who wrote a presence, its color, tags and web logo", () => {
  const root = presences({ alpha: {} });
  const manifestPath = join(root, "alpha", "manifest.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  Object.assign(manifest, { author: "Ana", color: "#A56786", tags: ["video"], icon: "https://img.example.com/a.png" });
  writeFileSync(manifestPath, JSON.stringify(manifest));
  const [entry] = JSON.parse(buildPayload({ root, commit: COMMIT })).presences;
  assert.deepEqual(
    [entry.author, entry.color, entry.tags, entry.icon],
    ["Ana", "#A56786", ["video"], "https://img.example.com/a.png"],
  );

  // A logo that is a file of the folder is not an address, so it is not listed as one.
  manifest.icon = "icon.png";
  writeFileSync(manifestPath, JSON.stringify(manifest));
  assert.equal(JSON.parse(buildPayload({ root, commit: COMMIT })).presences[0].icon, null);
});

test("the index carries the description of each language and the other names", () => {
  const root = presences({
    alpha: {
      files: {
        "locales/en.json": JSON.stringify({ description: "Shows the film." }),
        "locales/es.json": JSON.stringify({ description: "Muestra la película.", watching: "Viendo" }),
        "locales/pt-BR.json": JSON.stringify({ watching: "Assistindo" }),
      },
    },
    beta: {},
  });
  const manifestPath = join(root, "alpha", "manifest.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  manifest.other_names = ["AL", "Old Alpha"];
  writeFileSync(manifestPath, JSON.stringify(manifest));
  const [alpha, beta] = JSON.parse(buildPayload({ root, commit: COMMIT })).presences;
  assert.equal(alpha.description, "Shows the film.");
  assert.deepEqual(alpha.descriptions, { es: "Muestra la película." });
  assert.deepEqual(alpha.other_names, ["AL", "Old Alpha"]);
  assert.equal(beta.description, "Shows things.");
  assert.deepEqual([beta.descriptions, beta.other_names], [{}, []]);

  writeFileSync(join(root, "beta", "main.js"), "function read(){return null}");
  mkdirSync(join(root, "beta", "locales"), { recursive: true });
  writeFileSync(join(root, "beta", "locales", "es.json"), "{ not json");
  assert.throws(() => buildPayload({ root, commit: COMMIT }), /beta.locales.es\.json/);
});

test("a signature verifies, and fails after any edit or with another key", () => {
  const pem = key();
  const text = envelope(buildPayload({ root: presences({ alpha: {} }), commit: COMMIT }), pem, "main");
  assert.ok(check(text, publicKeyOf(pem)));
  assert.ok(!check(text, publicKeyOf(key())));
  const edited = JSON.parse(text);
  edited.payload = edited.payload.replace("1.0.0", "9.9.9");
  assert.ok(!check(JSON.stringify(edited), publicKeyOf(pem)));
  assert.equal(Buffer.from(publicKeyOf(pem), "base64").length, 32);
});

test("the index carries the format, and refuses one the application cannot read yet", () => {
  const root = presences({ alpha: {} });
  const file = join(root, "alpha", "manifest.json");
  const manifest = JSON.parse(readFileSync(file, "utf8"));
  const format = (value) => {
    writeFileSync(file, JSON.stringify({ ...manifest, format: value }));
    return JSON.parse(buildPayload({ root, commit: COMMIT })).presences[0].format;
  };
  assert.equal(JSON.parse(buildPayload({ root, commit: COMMIT })).presences[0].format, 1, "missing is format 1");
  assert.equal(format(1), 1);
  const newest = JSON.parse(
    readFileSync(new URL("../../crates/rpp-app/templates/manifest.schema.json", import.meta.url)),
  ).properties.format.maximum;
  for (const wrong of [0, newest + 1, 1.5, "1"]) {
    assert.throws(() => format(wrong), /format/, String(wrong));
  }
});

test("pre-releases stay out of the stable channel only", () => {
  const root = presences({ alpha: {}, beta: { version: "2.0.0-rc.1" } });
  const names = (channel) => JSON.parse(buildPayload({ root, commit: COMMIT, channel })).presences.map((p) => p.folder);
  assert.deepEqual(names("stable"), ["alpha"]);
  assert.deepEqual(names("experimental"), ["alpha", "beta"]);
});

test("a tag off the list is refused", () => {
  const root = presences({ alpha: {} });
  const file = join(root, "alpha", "manifest.json");
  const manifest = JSON.parse(readFileSync(file, "utf8"));
  writeFileSync(file, JSON.stringify({ ...manifest, tags: ["video"] }));
  assert.doesNotThrow(() => buildPayload({ root, commit: COMMIT }));
  writeFileSync(file, JSON.stringify({ ...manifest, tags: ["video", "invented"] }));
  assert.throws(() => buildPayload({ root, commit: COMMIT }), /invented/);
});

test("hidden files are skipped; unusual names, versions and folders are refused", () => {
  const root = presences({ alpha: { files: { ".DS_Store": "x" } } });
  assert.ok(!(".DS_Store" in JSON.parse(buildPayload({ root, commit: COMMIT })).presences[0].files));

  const build = (spec) => () => buildPayload({ root: presences(spec), commit: COMMIT });
  assert.throws(build({ alpha: { files: { "has space.js": "x" } } }), /unusual file name/);
  assert.throws(build({ alpha: { version: "1" } }), /not semver/);
  assert.throws(build({ Alpha: {} }), /folder names/);
  assert.throws(() => buildPayload({ root, commit: "main" }), /40-character/);
  assert.throws(() => buildPayload({ root, commit: COMMIT, channel: "beta" }), /stable or experimental/);
});

test("a bundle holds every file of a version and its name points back to it", () => {
  const root = presences({ "my-site": { version: "2.0.0", files: { "locales/en.json": '{"a":1}' } } });
  const payload = JSON.parse(buildPayload({ root, commit: COMMIT, generatedAt: 1, bundles: true }));
  const entry = payload.presences[0];
  const { name, text } = bundleOf(root, entry);
  assert.equal(entry.bundle, name, "the signed index names the bundle");
  const [, folder, version] = BUNDLE_NAME.exec(name);
  assert.deepEqual([folder, version], ["my-site", "2.0.0"]);
  const bundle = JSON.parse(text);
  assert.equal(bundle.format, "rich-presence-plus/bundle");
  assert.deepEqual(Object.keys(bundle.files), Object.keys(entry.files).sort());
  assert.equal(Buffer.from(bundle.files["locales/en.json"], "base64").toString(), '{"a":1}');
  // The same files give the same name; other files give another one.
  assert.equal(bundleOf(root, entry).name, name);
  writeFileSync(join(root, "my-site", "main.js"), "function read(){return 1}");
  assert.notEqual(bundleOf(root, entry).name, name);
});

test("without --bundles the index names no bundle", () => {
  const root = presences({ alpha: {} });
  const payload = JSON.parse(buildPayload({ root, commit: COMMIT, generatedAt: 1 }));
  assert.ok(!("bundle" in payload.presences[0]));
});

test("build metadata cannot be bundled, since GitHub renames the plus sign", () => {
  const root = presences({ alpha: { version: "1.0.0+b1" } });
  assert.throws(() => buildPayload({ root, commit: COMMIT, generatedAt: 1, bundles: true }), /build metadata/);
});
