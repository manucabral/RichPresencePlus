// Builds the zip uploaded to the Chrome Web Store, Edge Add-ons and
// addons.mozilla.org.
//
//   node tools/package-extension.mjs [--out dist] [--unpacked]
//
// With --unpacked it writes the copy people load by hand in a Chromium
// browser ("Load unpacked", developer mode) while the stores review the
// extension: the manifest keeps its `key`, so the copy has the ID the desktop
// app already allows and pairs by itself.
//
// Only the files the extension runs go in: no tests, no node_modules, no
// package files. The manifest loses its `key`, which fixes the ID of an
// unpacked copy for the desktop app but which the stores refuse; they assign
// their own IDs, added to the app's allowed list (crates/rpp-bridge/src/native.rs).

import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { pathToFileURL } from "node:url";

const EXTENSION = new URL("../extension/", import.meta.url);

/** The manifest the stores receive: the same, without `key`. */
export function storeManifest(manifest) {
  const { key: _key, ...rest } = manifest;
  return rest;
}

/** The manifest and zip name for the stores, or for loading by hand with `unpacked`. */
export function packageFor(manifest, unpacked) {
  return unpacked
    ? { manifest, zip: `rich-presence-plus-extension-${manifest.version}-unpacked.zip` }
    : { manifest: storeManifest(manifest), zip: `rich-presence-plus-extension-${manifest.version}.zip` };
}

/**
 * Files the extension runs, relative to its folder: the manifest, the popup,
 * the scripts at the top level, the translations and the icons.
 *
 * @param {string[]} all Every file in the folder, relative, with `/`.
 */
export function packagedFiles(all) {
  return all
    .filter(
      (path) =>
        path === "manifest.json" ||
        path === "popup.html" ||
        (/^[a-z]+\.js$/.test(path) && !path.endsWith(".test.js")) ||
        /^_locales\/[a-z_A-Z-]+\/messages\.json$/.test(path) ||
        /^icons\/icon-\d+\.png$/.test(path),
    )
    .sort();
}

/**
 * Files a set of scripts and the popup refer to, so a missing one is caught
 * before the upload instead of by a user.
 *
 * @param {Record<string, string>} sources Text of each packaged file by path
 *   (images may map to an empty string: only their presence is checked).
 * @returns {string[]} Referenced paths that are not in `sources`.
 */
export function missingReferences(sources) {
  const missing = new Set();
  for (const [path, text] of Object.entries(sources)) {
    const found = path.endsWith(".js")
      ? [...text.matchAll(/(?:from\s+|import\s+)"\.\/([^"]+)"/g)].map((m) => m[1])
      : path.endsWith(".html")
        ? [...text.matchAll(/<script[^>]+src="([^"]+)"/g)].map((m) => m[1])
        : [];
    for (const ref of found) if (!(ref in sources)) missing.add(ref);
  }
  if ("manifest.json" in sources) {
    const manifest = JSON.parse(sources["manifest.json"]);
    const named = [
      manifest.background?.service_worker,
      ...(manifest.background?.scripts ?? []),
      manifest.action?.default_popup,
      ...Object.values(manifest.icons ?? {}),
      ...Object.values(manifest.action?.default_icon ?? {}),
    ];
    for (const ref of named) if (ref && !(ref in sources)) missing.add(ref);
  }
  return [...missing].sort();
}

function walk(dir, base = dir) {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    if (name === "node_modules" || name === "test" || name.startsWith(".")) return [];
    return statSync(full).isDirectory() ? walk(full, base) : [relative(base, full).replaceAll("\\", "/")];
  });
}

/** A file's text; images are copied as they are, so only their name matters here. */
export function readText(path) {
  return path.endsWith(".png") ? "" : readFileSync(path, "utf8");
}

function main() {
  const outIndex = process.argv.indexOf("--out");
  const out = outIndex > 0 ? process.argv[outIndex + 1] : "dist";
  const source = EXTENSION.pathname.replace(/^\/([A-Za-z]:)/, "$1");
  const files = packagedFiles(walk(source));
  const sources = Object.fromEntries(files.map((file) => [file, readText(join(source, file))]));
  const missing = missingReferences(sources);
  if (missing.length > 0) throw new Error(`files referenced but not packaged: ${missing.join(", ")}`);

  const { manifest, zip: name } = packageFor(JSON.parse(sources["manifest.json"]), process.argv.includes("--unpacked"));
  const stage = join(out, "extension");
  rmSync(stage, { recursive: true, force: true });
  mkdirSync(stage, { recursive: true });
  for (const file of files) {
    mkdirSync(join(stage, file, ".."), { recursive: true });
    if (file === "manifest.json") writeFileSync(join(stage, file), `${JSON.stringify(manifest, null, 2)}\n`);
    else cpSync(join(source, file), join(stage, file));
  }
  // Listed one by one so the archive holds plain paths, manifest.json at the root.
  const zip = join(out, name);
  rmSync(zip, { force: true });
  const result =
    process.platform === "win32"
      ? // Windows' own bsdtar, by path: a GNU tar earlier on PATH reads "C:" as a host.
        spawnSync(
          join(process.env.SystemRoot ?? "C:Windows", "System32", "tar.exe"),
          ["-a", "-c", "-f", zip, "-C", stage, ...files],
          {
            stdio: "inherit",
          },
        )
      : spawnSync("zip", ["-X", join(process.cwd(), zip), ...files], { cwd: stage, stdio: "inherit" });
  if (result.status !== 0) throw new Error("could not write the zip");
  console.log(`${zip}: ${files.length} files`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main();
