/** The parts of a key press a shortcut is made of. */
export interface KeyPress {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  /** `KeyboardEvent.code`, such as `KeyP`, `Digit9` or `F12`: the key, whatever the layout. */
  code: string;
}

/**
 * Writes a key press the way the app stores shortcuts, such as `Ctrl+Alt+P`.
 *
 * @returns The shortcut, or null while only modifiers are held or the key is
 *     one a shortcut cannot use. The app checks it again before saving.
 */
export function shortcutOf(press: KeyPress): string | null {
  const letter = /^Key([A-Z])$/.exec(press.code);
  const digit = /^Digit([0-9])$/.exec(press.code);
  const fn = /^F([1-9]|1[0-9]|2[0-4])$/.exec(press.code);
  const key = letter?.[1] ?? digit?.[1] ?? (fn ? press.code : null);
  if (!key) return null;
  const parts = [
    press.ctrlKey && "Ctrl",
    press.altKey && "Alt",
    press.shiftKey && "Shift",
    press.metaKey && "Super",
  ].filter(Boolean) as string[];
  return [...parts, key].join("+");
}
