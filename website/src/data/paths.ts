/** Addresses of the pages, relative to the site's base. */
import type { Language } from "../i18n/strings";

/** The first page in a language. */
export function homePath(language: Language): string {
  return language === "en" ? "/" : `/${language}/`;
}

/** The list of presences in a language. */
export function presenceIndex(language: Language): string {
  return `${homePath(language)}presences/`;
}

/** The page of one presence in a language. */
export function presencePath(language: Language, folder: string): string {
  return `${presenceIndex(language)}${folder}/`;
}
