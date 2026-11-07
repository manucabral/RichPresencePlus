import { join } from "node:path";
import type { APIRoute } from "astro";
import { DOC_PAGES, docsPath } from "../data/docs";
import { presenceIndex, presencePath } from "../data/paths";
import { loadPresences } from "../data/presences";

const presences = loadPresences(join(process.cwd(), "..", "presences"));

/** The same page in each language, so search engines keep them together. */
const PAGES = [
  { en: "/", es: "/es/" },
  { en: presenceIndex("en"), es: presenceIndex("es") },
  { en: "/privacy/", es: "/es/privacy/" },
  { en: "/terms/", es: "/es/terms/" },
  { en: "/licenses/", es: "/es/licenses/" },
  { en: "/changelog/", es: "/es/changelog/" },
  ...DOC_PAGES.map(({ slug }) => ({ en: docsPath("en", slug), es: docsPath("es", slug) })),
  ...presences.map(({ folder }) => ({ en: presencePath("en", folder), es: presencePath("es", folder) })),
];

export const GET: APIRoute = ({ site }) => {
  const base = import.meta.env.BASE_URL.replace(/\/$/, "");
  const url = (path: string) => new URL(`${base}${path}`, site).href;
  const entries = PAGES.flatMap((page) =>
    Object.values(page).map((path) => {
      const links = Object.entries(page)
        .map(([language, other]) => `<xhtml:link rel="alternate" hreflang="${language}" href="${url(other)}"/>`)
        .join("");
      return `<url><loc>${url(path)}</loc>${links}</url>`;
    }),
  );
  const body = `<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">${entries.join("")}</urlset>`;
  return new Response(body, { headers: { "Content-Type": "application/xml; charset=utf-8" } });
};
