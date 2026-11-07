/** Texts of the developer tools, kept apart from the panels so they can be tested. */
import type { FileChange, ReviewCheck } from "../../shared/types.ts";

/** A size in bytes, for a list of versions. */
export function sizeText(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** Lines a file gained and lost; empty for a file that is not text. */
export function lineSummary(change: Pick<FileChange, "added" | "removed">): string {
  const parts = [];
  if (change.added > 0) parts.push(`+${change.added}`);
  if (change.removed > 0) parts.push(`−${change.removed}`);
  return parts.join(" ");
}

/** The parts of the Developer page. */
export type DeveloperTab = "create" | "code" | "test" | "publish";

/**
 * Chooses the tab a visit starts on: testing when a presence page sent the
 * user here to inspect one, coding when the user has a presence of their own,
 * creating otherwise.
 */
export function firstTab(hasOwn: boolean, linked: boolean): DeveloperTab {
  if (linked) return "test";
  return hasOwn ? "code" : "create";
}

/** Whether a presence follows every rule, once the rules have loaded. */
export function allPassed(checks: Pick<ReviewCheck, "passed">[]): boolean {
  return checks.length > 0 && checks.every((check) => check.passed);
}
