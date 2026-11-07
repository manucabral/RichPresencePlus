/**
 * How the Dashboard draws each presence: a state icon picked from the
 * policy's reason, and priority as one to three bars.
 */

/** Source name of activities written by hand (`rpp-app/src/manual.rs`). */
export const MANUAL_SOURCE = ".manual";

/** Manual activities have their own page; presences open theirs. */
export function optionsLink(source: string): string {
  return source === MANUAL_SOURCE ? "/manual" : `/presences/${encodeURIComponent(source)}`;
}

/** The look of a row's state icon. */
export type Tone = "showing" | "waiting" | "paused" | "standby" | "hidden" | "stopped";

/** Reasons written by `rpp-core/src/policy.rs`, by the tone they get. */
const BY_REASON: Record<string, Tone> = {
  "Waiting before switching": "waiting",
  "Media is paused": "paused",
  "Another activity has a higher priority; on a tie the current one stays": "standby",
  "Presence stopped reporting": "stopped",
  "Publishing is paused": "hidden",
  "Disabled for this presence": "hidden",
  "Application excluded by privacy preferences": "hidden",
  "Domain excluded by privacy preferences": "hidden",
  "Hidden while the computer is idle": "hidden",
  "Hidden by your automatic rules": "hidden",
};

/** Picks the icon of a row; an unknown reason counts as hidden. */
export function toneOf(reason: string, selected: boolean): Tone {
  if (selected) return "showing";
  return Object.hasOwn(BY_REASON, reason) ? BY_REASON[reason] : "hidden";
}

/** Turns a priority from 0 to 100 into filled bars: 1 up to 33, 2 up to 66, 3 above. */
export function priorityBars(priority: number): 1 | 2 | 3 {
  if (priority > 66) return 3;
  return priority > 33 ? 2 : 1;
}

/** Rows to draw: the one on display first, then by priority, then by name. */
export function ordered<T extends { selected: boolean; priority: number; label: string }>(rows: readonly T[]): T[] {
  return [...rows].sort(
    (a, b) =>
      Number(b.selected) - Number(a.selected) || b.priority - a.priority || a.label.localeCompare(b.label),
  );
}
