import type { Language } from "../i18n/strings";
import { DOC_PAGES, docsPath } from "./docs";

interface Doc {
  frontmatter: { title: string };
  getHeadings: () => { depth: number; slug: string; text: string }[];
}

const files = import.meta.glob<Doc>("../../../docs/developers/*/*.md", { eager: true });

/** One entry the docs search can find: a page title or one of its headings. */
export interface SearchEntry {
  /** What the reader sees in the results. */
  text: string;
  /** Title of the page that holds it, empty for the title itself. */
  page: string;
  /** Path inside the site, with the heading's anchor. */
  path: string;
}

/** Every page title and section heading of one language, for the docs search. */
export function searchIndex(language: Language): SearchEntry[] {
  return DOC_PAGES.flatMap(({ slug, file }) => {
    const found = Object.entries(files).find(([path]) => path.endsWith(`/${language}/${file}.md`));
    if (!found) throw new Error(`no ${language} page ${file}`);
    const doc = found[1];
    const path = docsPath(language, slug);
    const title = doc.frontmatter.title;
    const headings = doc
      .getHeadings()
      .filter((heading) => heading.depth === 2 || heading.depth === 3)
      .map((heading) => ({ text: heading.text, page: title, path: `${path}#${heading.slug}` }));
    return [{ text: title, page: "", path }, ...headings];
  });
}
