import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { fill, merged, normalize, pick, plural, review, translate } from "../src/shared/i18n/core.ts";
import { reasonText } from "../src/features/dashboard/reason.ts";

test("tags are normalized like the Rust side", () => {
  assert.equal(normalize("es_ar"), "es-AR");
  assert.equal(normalize("EN"), "en");
  assert.equal(normalize("pt_BR.UTF-8"), "pt-BR");
  assert.equal(normalize("zh-hans"), "zh-hans");
  assert.equal(normalize(""), null);
  assert.equal(normalize("../x"), null);
  assert.equal(normalize("es--AR"), null);
});

test("the closest language is picked", () => {
  const available = ["en", "es", "pt-BR"];
  assert.equal(pick("es-AR", available), "es");
  assert.equal(pick("pt", available), "pt-BR");
  assert.equal(pick("fr", available), "en");
  assert.equal(pick("garbage!", available), "en");
  assert.equal(pick("fr", ["es"]), null);
});

test("placeholders are filled and unknown ones stay visible", () => {
  assert.equal(fill("{who} has {n}", { who: "Ana", n: 3 }), "Ana has 3");
  assert.equal(fill("{missing}", { n: 1 }), "{missing}");
  assert.equal(translate({}, "no.such.key"), "no.such.key");
});

test("plural forms follow the language rules and fall back to other", () => {
  const table = { "n_one": "{count} file", "n_other": "{count} files" };
  assert.equal(plural(table, "en", "n", 1), "1 file");
  assert.equal(plural(table, "en", "n", 5), "5 files");
  // Spanish has no "few"; a language that does falls back to other.
  assert.equal(plural({ "n_other": "{count} x" }, "ru", "n", 3), "3 x");
});

test("a translation overlays English key by key", () => {
  const tables = { en: { a: "A", b: "B" }, es: { a: "Uno" } };
  assert.deepEqual(merged(tables, "es"), { a: "Uno", b: "B" });
  assert.deepEqual(merged(tables, "en"), { a: "A", b: "B" });
});

test("review finds unknown keys, empty texts and changed placeholders", () => {
  const base = { hi: "Hello {who}", n_one: "{count} item", n_other: "{count} items" };
  assert.deepEqual(review(base, { hi: "Hola {who}" }), []);
  assert.equal(review(base, { hi: "Hola {quien}" }).length, 1);
  assert.equal(review(base, { nope: "x" }).length, 1);
  assert.ok(review(base, { hi: "  " }).some((problem) => problem.includes("empty")));
  // Languages with more plural categories may add forms English lacks.
  assert.deepEqual(review(base, { n_few: "{count} cosas" }), []);
});

test("every shipped locale passes the review", () => {
  const dir = join(import.meta.dirname, "../src/locales");
  const base = JSON.parse(readFileSync(join(dir, "en.json"), "utf8"));
  for (const file of readdirSync(dir).filter((name) => name.endsWith(".json"))) {
    const table = JSON.parse(readFileSync(join(dir, file), "utf8"));
    assert.deepEqual(review(base, table), [], file);
  }
});

test("every key the code uses exists in English", () => {
  const root = join(import.meta.dirname, "../src");
  const base = JSON.parse(readFileSync(join(root, "locales/en.json"), "utf8"));
  const walk = (dir: string): string[] =>
    readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
      entry.isDirectory() ? walk(join(dir, entry.name)) : /\.(ts|tsx)$/.test(entry.name) ? [join(dir, entry.name)] : [],
    );
  const used = new Set<string>();
  for (const file of walk(root)) {
    const text = readFileSync(file, "utf8");
    for (const match of text.matchAll(/\bt[n]?\(\s*"([a-zA-Z]+\.[a-zA-Z.]+)"/g)) used.add(match[1]);
    for (const match of text.matchAll(/label: "([a-z]+\.[a-zA-Z]+)"/g)) used.add(match[1]);
  }
  for (const key of used) {
    const found = key in base || `${key}_other` in base;
    assert.ok(found, `missing in en.json: ${key}`);
  }
  assert.ok(used.size > 40);
});

test("keys that only exist in plural forms are read with tn", () => {
  const root = join(import.meta.dirname, "../src");
  const base = JSON.parse(readFileSync(join(root, "locales/en.json"), "utf8"));
  const walk = (dir: string): string[] =>
    readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
      entry.isDirectory() ? walk(join(dir, entry.name)) : /\.(ts|tsx)$/.test(entry.name) ? [join(dir, entry.name)] : [],
    );
  for (const file of walk(root)) {
    const text = readFileSync(file, "utf8");
    // `t("x")` with only `x_one`/`x_other` defined shows the key itself.
    for (const match of text.matchAll(/(?<![\w.])t\(\s*"([a-zA-Z]+\.[a-zA-Z.]+)"/g)) {
      const key = match[1];
      assert.ok(key in base || !(`${key}_other` in base), `${file}: use tn() for ${key}`);
    }
  }
});

test("policy reasons are translated and unknown ones pass through", () => {
  const policy = readFileSync(join(import.meta.dirname, "../../crates/rpp-core/src/policy.rs"), "utf8");
  const table = JSON.parse(readFileSync(join(import.meta.dirname, "../src/locales/en.json"), "utf8"));
  const t = (key: string, values?: Record<string, string | number>) => translate(table, key, values);
  for (const key of Object.keys(table).filter((k) => k.startsWith("reason.") && k !== "reason.selected")) {
    const english = table[key];
    // The text the policy writes is the English text of the key.
    assert.ok(policy.includes(`"${english}"`), `policy.rs no longer says: ${english}`);
    assert.equal(reasonText(english, t), english);
  }
  assert.equal(reasonText("Selected: Video, priority 60", t), "Selected: video, priority 60");
  assert.equal(reasonText("Something new", t), "Something new");
});
