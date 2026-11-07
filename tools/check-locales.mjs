// Reviews every translation in the repository: the interface
// (frontend/src/locales) and each presence (presences/<name>/locales).
// Run by contributors before a pull request and by CI on every change.
//
//   node tools/check-locales.mjs
//
// A translation may be incomplete (missing keys fall back to English); what
// fails the check is a wrong tag, an unreadable file, a key English does not
// have, a changed placeholder, or an English text a script or page uses but
// does not define.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { normalize, review } from "../frontend/src/shared/i18n/core.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const failures = [];
const lines = [];

function readTable(file, label) {
  let table;
  try {
    table = JSON.parse(readFileSync(file, "utf8"));
  } catch (error) {
    failures.push(`${label}: ${error.message}`);
    return null;
  }
  const flat =
    table !== null &&
    typeof table === "object" &&
    !Array.isArray(table) &&
    Object.values(table).every((value) => typeof value === "string");
  if (!flat) {
    failures.push(`${label}: must be one object of text values, with no nesting`);
    return null;
  }
  return table;
}

function checkFolder(folder, label, usedKeys) {
  if (!existsSync(folder)) return;
  const english = readTable(join(folder, "en.json"), `${label}/en.json`);
  if (!english) {
    if (!existsSync(join(folder, "en.json"))) failures.push(`${label}: en.json is required`);
    return;
  }
  for (const key of usedKeys ?? []) {
    if (!(key in english) && !(`${key}_other` in english))
      failures.push(`${label}/en.json: the code uses "${key}" but English lacks it`);
  }
  const total = Object.keys(english).length;
  for (const name of readdirSync(folder)
    .filter((n) => n.endsWith(".json"))
    .sort()) {
    const tag = name.slice(0, -5);
    if (normalize(tag) !== tag) {
      failures.push(`${label}/${name}: the file name must be a language tag such as es or pt-BR`);
      continue;
    }
    const table = readTable(join(folder, name), `${label}/${name}`);
    if (!table) continue;
    for (const problem of review(english, table)) failures.push(`${label}/${name}: ${problem}`);
    if (tag !== "en") {
      const done = Object.keys(table).filter((key) => key in english).length;
      lines.push(`${label}/${tag}: ${done}/${total}`);
    }
  }
}

function keysIn(text, pattern) {
  return [...text.matchAll(pattern)].map((match) => match[1]);
}

// The interface.
const frontend = join(root, "frontend/src");
const sources = [];
(function walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) walk(path);
    else if (/\.(ts|tsx)$/.test(entry.name)) sources.push(path);
  }
})(frontend);
const interfaceKeys = new Set();
for (const file of sources) {
  const text = readFileSync(file, "utf8");
  for (const key of keysIn(text, /\bt[n]?\(\s*"([a-zA-Z]+\.[a-zA-Z.]+)"/g)) interfaceKeys.add(key);
  for (const key of keysIn(text, /label: "([a-z]+\.[a-zA-Z]+)"/g)) interfaceKeys.add(key);
}
checkFolder(join(frontend, "locales"), "interface", interfaceKeys);

// The presences.
const presences = join(root, "presences");
for (const name of readdirSync(presences, { withFileTypes: true }).filter((e) => e.isDirectory())) {
  const folder = join(presences, name.name);
  const script = join(folder, "main.js");
  const used = existsSync(script) ? keysIn(readFileSync(script, "utf8"), /\bt\(\s*"([A-Za-z0-9_.-]+)"/g) : [];
  if (used.length > 0 && !existsSync(join(folder, "locales"))) {
    failures.push(`presences/${name.name}: the script calls t() but there is no locales folder`);
  }
  checkFolder(join(folder, "locales"), `presences/${name.name}`, used);
}

for (const line of lines) console.log(line);
if (failures.length > 0) {
  console.error(`\n${failures.length} problem(s):`);
  for (const failure of failures) console.error(`  ${failure}`);
  process.exit(1);
}
console.log("Translations are sound.");
