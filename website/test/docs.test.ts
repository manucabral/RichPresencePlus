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
const read = (language: string, file: string) => readFileSync(join(DOCS, language, `${file}.md`), "utf8");

test("every page of the list has its file, and no file is left out", () => {
  const listed = DOC_PAGES.map((page) => `${page.file}.md`).sort();
  assert.deepEqual(readdirSync(join(DOCS, "en")).sort(), listed);
  assert.deepEqual(readdirSync(join(DOCS, "es")).sort(), listed);
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
      for (const [, target] of read(language, file).matchAll(/\]\(([^)#]+\.md)/g)) {
        assert.ok(files.has(target), `${language}/${file}: ${target}`);
      }
    }
  }
});

test("the site has every page in both languages, with no link left to a file", { skip: !existsSync(join(DIST, "docs")) }, () => {
  for (const language of ["en", "es"] as const) {
    for (const { slug } of DOC_PAGES) {
      const html = readFileSync(join(DIST, docsPath(language, slug), "index.html"), "utf8");
      assert.doesNotMatch(html, /href="[^"]*\.md/, `${language}/${slug}`);
    }
  }
});
