import type { Language } from "../i18n/strings";

/** The four parts of the documentation, in reading order. */
export const DOC_SECTIONS = ["start", "guides", "reference", "inside"] as const;

/** A part of the documentation. */
export type DocSection = (typeof DOC_SECTIONS)[number];

/** The pages of the developer documentation, in reading order. */
export const DOC_PAGES = [
  { slug: "", file: "01-home", section: "start" },
  { slug: "first-presence", file: "02-first-presence", section: "start" },
  { slug: "read-a-site", file: "03-read-a-site", section: "guides" },
  { slug: "desktop", file: "04-desktop", section: "guides" },
  { slug: "options", file: "05-options", section: "guides" },
  { slug: "troubleshoot", file: "06-troubleshoot", section: "guides" },
  { slug: "publish", file: "07-publish", section: "guides" },
  { slug: "manifest", file: "08-manifest", section: "reference" },
  { slug: "extractors", file: "09-extractors", section: "reference" },
  { slug: "script", file: "10-script", section: "reference" },
  { slug: "host", file: "11-host", section: "reference" },
  { slug: "helpers", file: "12-helpers", section: "reference" },
  { slug: "limits", file: "13-limits", section: "reference" },
  { slug: "how-it-works", file: "14-how-it-works", section: "inside" },
] as const satisfies readonly { slug: string; file: string; section: DocSection }[];

/** The path inside the site of a documentation page; an empty slug is the first page. */
export function docsPath(language: Language, slug: string): string {
  const start = language === "en" ? "/docs/" : `/${language}/docs/`;
  return slug === "" ? start : `${start}${slug}/`;
}

/** The slug of a page by its file name, such as `08-manifest.md`, or null when none has it. */
export function slugOfFile(file: string): string | null {
  const name = file.replace(/\.md$/, "");
  return DOC_PAGES.find((page) => page.file === name)?.slug ?? null;
}

/** The path inside the site of the practice page used by the tutorial. */
export function practicePath(language: Language): string {
  return language === "en" ? "/practice/" : `/${language}/practice/`;
}
