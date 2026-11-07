import type { ActivityButton, ManualPreset } from "../../shared/types";

/** Buttons Discord allows on one activity. */
export const MAX_BUTTONS = 2;

export function blankPreset(): ManualPreset {
  return {
    name: "",
    client_id: "",
    show_elapsed: false,
    activity: {
      state: null,
      details: null,
      kind: "playing",
      start: null,
      end: null,
      large_image: null,
      large_text: null,
      small_image: null,
      small_text: null,
      buttons: [],
    },
  };
}

/** The text of a field, or null when it is empty, as the Rust side expects. */
export function orNull(text: string): string | null {
  return text.trim() === "" ? null : text;
}

/** The buttons of a form, always `MAX_BUTTONS` long so each has its own row. */
export function buttonRows(buttons: ActivityButton[]): ActivityButton[] {
  return Array.from({ length: MAX_BUTTONS }, (_, index) => buttons[index] ?? { label: "", url: "" });
}

/** The rows that were filled in, ready to save. */
export function filledButtons(rows: ActivityButton[]): ActivityButton[] {
  return rows.filter((row) => row.label.trim() !== "" || row.url.trim() !== "");
}
