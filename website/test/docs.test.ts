/**
 * Checks of the developer documentation: both languages say the same, and each link
 * lands on a page and a heading that exist. Run `pnpm build` first for the last one.
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { DOC_PAGES, docsPath } from "../src/data/docs.ts";

const DOCS = new URL("../../docs/developers/", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const DIST = new URL("../dist/", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const TEMPLATES = new URL("../../crates/rpp-app/templates/", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const read = (language: string, file: string) => readFileSync(join(DOCS, language, `${file}.md`), "utf8");
const LANGUAGES = ["en", "es"] as const;

test("every page of the list has its file, and no file is left out", () => {
  const listed = DOC_PAGES.map((page) => `${page.file}.md`).sort();
  for (const language of LANGUAGES) {
    const files = readdirSync(join(DOCS, language)).filter((name) => name.endsWith(".md"));
    assert.deepEqual(files.sort(), listed, language);
  }
});

test("both languages have the same parts and the same code", () => {
  const headings = (text: string) => text.match(/^## /gm)?.length ?? 0;
  const code = (text: string) => text.match(/^```/gm)?.length ?? 0;
  for (const { file } of DOC_PAGES) {
    const [en, es] = [read("en", file), read("es", file)];
    assert.equal(headings(es), headings(en), `${file}: headings`);
    assert.equal(code(es), code(en), `${file}: code blocks`);
  }
});

test("a link between pages names a page that exists", () => {
  const files = new Set(DOC_PAGES.map((page) => `${page.file}.md`));
  for (const language of ["en", "es"]) {
    for (const { file } of DOC_PAGES) {
      for (const [, target] of read(language, file).matchAll(/\]\((\d{2}-[^)#]+\.md)/g)) {
        assert.ok(files.has(target), `${language}/${file}: ${target}`);
      }
    }
  }
});

/** The names written in code font on a page, such as `client_id`. */
const named = (text: string) => {
  const prose = text.replace(/^```[\s\S]*?^```/gm, "");
  return new Set([...prose.matchAll(/`([^`]+)`/g)].map((found) => found[1]));
};

test("the manifest reference names every field the schema knows", () => {
  const schema = JSON.parse(readFileSync(join(TEMPLATES, "manifest.schema.json"), "utf8"));
  const groups = [
    { file: "08-manifest", fields: Object.keys(schema.properties) },
    { file: "08-manifest", fields: Object.keys(schema.properties.page.properties) },
    { file: "08-manifest", fields: Object.keys(schema.definitions.setting.properties) },
    { file: "11-host", fields: Object.keys(schema.properties.permissions.properties) },
    { file: "08-manifest", fields: schema.properties.tags.items.enum },
  ];
  const sources: string[] = schema.definitions.extractor.oneOf.map((option: { properties: { from: { const: string } } }) => option.properties.from.const);
  for (const language of LANGUAGES) {
    for (const { file, fields } of groups) {
      const names = named(read(language, file));
      for (const field of fields) assert.ok(names.has(field), `${language}/${file}: ${field}`);
    }
    const extractors = read(language, "09-extractors");
    for (const from of sources) assert.match(extractors, new RegExp(`^## ${from}$`, "m"), `${language}: extractor ${from}`);
  }
});

test("the script reference names every field and helper the editor knows", () => {
  const types = readFileSync(join(TEMPLATES, "script-api.d.ts"), "utf8");
  const fields = (name: string) => {
    const start = types.indexOf(`interface ${name} {`);
    const body = start < 0 ? "" : types.slice(start, types.indexOf("\n}", start));
    return [...body.matchAll(/^ {2}([a-z_]+)\??:/gm)].map((found) => found[1]);
  };
  const helpers = [...types.matchAll(/^declare function ([A-Za-z]+)\(([^)]*)\)/gm)];
  assert.ok(helpers.length >= 7, "helpers found in the types");
  for (const language of LANGUAGES) {
    const script = named(read(language, "10-script"));
    for (const name of ["Context", "Page", "Activity", "Meta"]) {
      const list = fields(name);
      assert.ok(list.length > 0, `fields of ${name}`);
      for (const field of list) assert.ok(script.has(field), `${language}/10-script: ${name}.${field}`);
    }
    const page = read(language, "12-helpers");
    for (const [, name] of helpers) assert.match(page, new RegExp(`^## ${name}[(]`, "m"), `${language}/12-helpers: ${name}`);
  }
});

// Words that keep their capital inside a heading.
const PROPER = new Set(["Windows", "Discord", "JavaScript", "GitHub", "VS", "Code", "Rich", "Presence", "Plus", "Unix", "SA-MP"]);

test("headings use a capital only at the start", () => {
  for (const language of LANGUAGES) {
    for (const { file } of DOC_PAGES) {
      for (const [, heading] of read(language, file).matchAll(/^#{1,3} (.+)$/gm)) {
        const words = heading.replace(/`[^`]*`/g, "").replace(/^\d+\.\s*/, "").split(/\s+/).slice(1);
        const capital = words.filter((word) => /^[A-ZÁÉÍÓÚÑ]/.test(word) && !PROPER.has(word.replace(/[^\w-]/g, "")));
        assert.equal(capital.length, 0, `${language}/${file}: "${heading}"`);
      }
    }
  }
});

test("the site has every page in both languages, and each link lands on a heading that exists", { skip: !existsSync(join(DIST, "docs")) }, () => {
  const html = (path: string) => readFileSync(join(DIST, path, "index.html"), "utf8");
  const base = "/RichPresencePlus";
  for (const language of LANGUAGES) {
    for (const { slug } of DOC_PAGES) {
      const page = html(docsPath(language, slug));
      assert.doesNotMatch(page, /href="(?!https:)[^"]*\.md/, `${language}/${slug}: a link left to a file`);
      for (const [, target, anchor] of page.matchAll(/href="([^"#]*\/docs\/[^"#]*)#([^"]+)"/g)) {
        const path = target.startsWith(base) ? target.slice(base.length) : target;
        const landing = path === "" ? page : html(path);
        assert.ok(landing.includes(`id="${anchor}"`), `${language}/${slug}: ${target}#${anchor}`);
      }
    }
  }
});

test("every picture icon() names is a 256 pixel PNG of the site", () => {
  const prelude = readFileSync(new URL("../../crates/rpp-plugins/src/prelude.js", import.meta.url), "utf8");
  const listed = /const ICONS = \[([^\]]+)\]/.exec(prelude);
  assert.ok(listed, "ICONS in prelude.js");
  const names = [...listed[1].matchAll(/"([a-z-]+)"/g)].map((found) => found[1]);
  assert.ok(names.length > 0);
  const address = /icons\/\$\{name\}\.png/.test(prelude) && /https:\/\/manucabral\.github\.io\/RichPresencePlus\//.test(prelude);
  assert.ok(address, "icon() points at the icons folder of this site");
  for (const name of names) {
    const file = new URL(`../public/icons/${name}.png`, import.meta.url);
    assert.ok(existsSync(file), `${name}.png`);
    const png = readFileSync(file);
    assert.equal(png.subarray(1, 4).toString(), "PNG", name);
    assert.deepEqual([png.readUInt32BE(16), png.readUInt32BE(20)], [256, 256], name);
  }
});
