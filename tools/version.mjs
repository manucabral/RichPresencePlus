// Checks that the program has one version, and that a release tag names it.
//
//   node tools/version.mjs [--tag v1.2.3]
//
// The version is written in Cargo.toml (the workspace) and in
// src-tauri/tauri.conf.json (the installer and the updater). The updater
// compares the running program's version with latest.json, which takes the
// tag's, so a tag that differs would offer the same build again and again.

import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const ROOT = new URL("..", import.meta.url);

/** The workspace version in Cargo.toml. */
export function cargoVersion(text) {
  const workspace = text.split(/^\[workspace\.package\]$/m)[1] ?? "";
  const found = /^version\s*=\s*"([^"]+)"/m.exec(workspace.split(/^\[/m)[0]);
  if (!found) throw new Error("Cargo.toml has no [workspace.package] version");
  return found[1];
}

/** The version, or an error naming what does not match. */
export function check({ cargo, config, tag }) {
  const version = cargoVersion(cargo);
  const installer = JSON.parse(config).version;
  if (installer !== version) throw new Error(`Cargo.toml says ${version} and tauri.conf.json says ${installer}`);
  if (tag !== undefined && tag !== `v${version}`)
    throw new Error(`the tag ${tag} is not v${version}, the program's version`);
  return version;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  const at = process.argv.indexOf("--tag");
  const tag = at === -1 ? undefined : process.argv[at + 1];
  try {
    const version = check({
      cargo: readFileSync(new URL("Cargo.toml", ROOT), "utf8"),
      config: readFileSync(new URL("src-tauri/tauri.conf.json", ROOT), "utf8"),
      tag,
    });
    console.log(`version ${version}${tag ? `, tag ${tag}` : ""}`);
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}
