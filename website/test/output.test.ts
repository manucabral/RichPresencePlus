/**
 * Checks of the built site: what search engines read and how much a visitor
 * downloads. Run `pnpm build` first; without a build these tests are skipped.
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";

const DIST = new URL("../dist/", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const built = existsSync(join(DIST, "index.html"));

/** Largest HTML page, styles included, before compression. */
const MAX_PAGE = 48 * 1024;
/** The same for a page of the documentation, which is mostly text and code. */
const MAX_DOC_PAGE = 64 * 1024;

function pages(dir = DIST): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return pages(path);
    return name.endsWith(".html") ? [path] : [];
  });
}

// Pages kept away from search engines, such as 404 and the tutorial's practice page.
const indexed = () =>
  pages().filter((path) => !path.endsWith("404.html") && !readFileSync(path, "utf8").includes('content="noindex"'));

test("every page stays within its size and runs only its own script", { skip: !built }, () => {
  for (const path of pages()) {
    const html = readFileSync(path, "utf8");
    const limit = path.replaceAll("\\", "/").includes("/docs/") ? MAX_DOC_PAGE : MAX_PAGE;
    assert.ok(html.length <= limit, `${path}: ${html.length} bytes`);
    const scripts = html.match(/<script[^>]*>/g) ?? [];
    const own = scripts.filter((tag) => !tag.includes("application/ld+json"));
    const inner = path.slice(DIST.length).replace(/\\/g, "/").replace(/^\//, "");
    const allowed = /^(es\/)?presences\/index\.html$/.test(inner)
      ? "/search.js"
      : /^(es\/)?index\.html$/.test(inner)
        ? "/hero.js"
        : /^(es\/)?docs\//.test(inner)
          ? "/docs.js"
          : null;
    assert.ok(own.every((tag) => allowed && tag.includes(allowed)), `${inner}: ${own}`);
  }
});

test("nothing loads from another server", { skip: !built }, () => {
  for (const path of pages()) {
    const html = readFileSync(path, "utf8");
    const loaded = [...html.matchAll(/<(?:img|link rel="(?:stylesheet|preload)"|iframe)[^>]*(?:src|href)="(https?:[^"]+)"/g)];
    assert.deepEqual(loaded.map((match) => match[1]), [], path);
  }
});

test("every page says what search engines need", { skip: !built }, () => {
  const titles = new Set<string>();
  for (const path of indexed()) {
    const html = readFileSync(path, "utf8");
    const title = /<title>([^<]+)<\/title>/.exec(html)?.[1] ?? "";
    assert.ok(title.length > 10 && title.length <= 70, `${path}: title of ${title.length}`);
    assert.ok(!titles.has(title), `repeated title: ${title}`);
    titles.add(title);
    const description = /<meta name="description" content="([^"]+)"/.exec(html)?.[1] ?? "";
    assert.ok(description.length >= 70 && description.length <= 170, `${path}: description of ${description.length}`);
    assert.equal(html.match(/<h1[ >]/g)?.length, 1, `${path}: one h1`);
    assert.match(html, /<link rel="canonical" href="https:/, path);
    assert.match(html, /hreflang="en"/, path);
    assert.match(html, /hreflang="es"/, path);
    assert.match(html, /hreflang="x-default"/, path);
    assert.match(html, /property="og:image"/, path);
  }
});

test("the sitemap names every page", { skip: !built }, () => {
  const sitemap = readFileSync(join(DIST, "sitemap.xml"), "utf8");
  assert.equal(sitemap.match(/<loc>/g)?.length, indexed().length);
});

test("every link inside the site lands on a page or file that exists", { skip: !built }, () => {
  const base = "/RichPresencePlus";
  for (const path of pages()) {
    const html = readFileSync(path, "utf8");
    for (const [, href] of html.matchAll(/(?:href|src)="(\/[^"#?]*)/g)) {
      if (!href.startsWith(`${base}/`)) continue;
      const inside = href.slice(base.length).replace(/^\//, "");
      const target = join(DIST, inside);
      const exists = href.endsWith("/") ? existsSync(join(target, "index.html")) : existsSync(target);
      assert.ok(exists, `${path}: ${href}`);
    }
  }
});
