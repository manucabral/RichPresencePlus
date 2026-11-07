import type { Language } from "../i18n/strings";

/** The pages of the developer documentation, in reading order. */
export const DOC_PAGES = [
  { slug: "", file: "01-home" },
  { slug: "setup", file: "02-setup" },
  { slug: "structure", file: "03-structure" },
  { slug: "functions", file: "04-functions" },
  { slug: "web", file: "05-web" },
  { slug: "desktop", file: "06-desktop" },
  { slug: "publish", file: "07-publish" },
] as const;

/** The path inside the site of a documentation page; an empty slug is the first page. */
export function docsPath(language: Language, slug: string): string {
  const start = language === "en" ? "/docs/" : `/${language}/docs/`;
  return slug === "" ? start : `${start}${slug}/`;
}
