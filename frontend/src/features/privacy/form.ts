import type { GlobalPreferences } from "../../shared/types";

/** The entries of a text box with one per line. */
export const lines = (text: string) =>
  text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);

/**
 * True when the form says something other than what is saved. The three
 * texts are the boxes of excluded sites, excluded programs and programs that
 * hide everything.
 */
export function edited(saved: GlobalPreferences, form: GlobalPreferences, domains: string, applications: string, quiet: string) {
  const current = { ...form, excluded_domains: lines(domains), excluded_applications: lines(applications), quiet_programs: lines(quiet) };
  return JSON.stringify(current) !== JSON.stringify(saved);
}

/** Writes minutes after midnight as `HH:MM`, the way a time box reads them. */
export function clock(minutes: number): string {
  const part = (value: number) => String(value).padStart(2, "0");
  return `${part(Math.floor(minutes / 60))}:${part(minutes % 60)}`;
}

/** Reads an `HH:MM` time box as minutes after midnight; null while it is empty. */
export function minutesOf(text: string): number | null {
  const found = /^(\d{2}):(\d{2})$/.exec(text);
  return found ? Number(found[1]) * 60 + Number(found[2]) : null;
}
