// Fails a pull request that changes a presence without raising its version.
// Installed copies update only when the version goes up, so a change that
// keeps it would never reach anyone.
//
//   node tools/check-versions.mjs <base-ref>
//
// A new presence and a removed one pass. So does a presence whose only change
// is outside its folder.
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

/** Splits "1.2.3" into numbers; anything else gives null. */
function parse(version) {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(String(version ?? ""));
  return match ? match.slice(1).map(Number) : null;
}

/** True when version `next` is higher than `previous`. */
export function isHigher(next, previous) {
  const a = parse(next);
  const b = parse(previous);
  if (!a || !b) return false;
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i] > b[i];
  }
  return false;
}

/** Presence folders named by a list of changed paths, each once. */
export function changedPresences(paths) {
  const names = new Set();
  for (const path of paths) {
    const match = /^presences\/([^/]+)\//.exec(path);
    if (match) names.add(match[1]);
  }
  return [...names].sort();
}

/**
 * Lists the problems for each changed presence.
 *
 * `before` and `after` map a folder name to its manifest text, or to null when
 * the folder has no manifest on that side.
 */
export function review(names, before, after) {
  const problems = [];
  for (const name of names) {
    const old = before[name];
    const now = after[name];
    if (old == null || now == null) continue;
    let oldVersion;
    let newVersion;
    try {
      oldVersion = JSON.parse(old).version;
      newVersion = JSON.parse(now).version;
    } catch {
      problems.push(`${name}: manifest.json is not valid JSON`);
      continue;
    }
    if (!isHigher(newVersion, oldVersion)) {
      problems.push(`${name}: changed but its version is ${newVersion}; raise it above ${oldVersion}`);
    }
  }
  return problems;
}

function show(ref, path) {
  try {
    return execFileSync("git", ["show", `${ref}:${path}`], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  } catch {
    return null;
  }
}

function main(base) {
  if (!base) {
    console.error("usage: node tools/check-versions.mjs <base-ref>");
    process.exit(2);
  }
  const paths = execFileSync("git", ["diff", "--name-only", `${base}...HEAD`, "--", "presences"], {
    encoding: "utf8",
  })
    .split("\n")
    .filter(Boolean);
  const names = changedPresences(paths);
  const before = {};
  const after = {};
  for (const name of names) {
    before[name] = show(base, `presences/${name}/manifest.json`);
    after[name] = show("HEAD", `presences/${name}/manifest.json`);
  }
  const problems = review(names, before, after);
  for (const problem of problems) console.error(problem);
  if (problems.length) process.exit(1);
  console.log(`${names.length} changed presence(s), versions raised`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main(process.argv[2]);
