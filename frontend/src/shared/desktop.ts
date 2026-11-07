/** Keys the web view would treat as a browser's: reload, print, source, find. */
const BROWSER_KEYS = new Set(["f3", "f5", "f7", "f12", "ctrl+f", "ctrl+g", "ctrl+j", "ctrl+n", "ctrl+o", "ctrl+p", "ctrl+r", "ctrl+s", "ctrl+t", "ctrl+u", "ctrl+shift+i", "ctrl+shift+j", "ctrl+shift+r", "ctrl+shift+s"]);

/**
 * Names a key press the way {@link BROWSER_KEYS} lists it: the key in lower
 * case, with its modifiers in front.
 */
export function keyName(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey">): string {
  const modifiers = `${event.ctrlKey || event.metaKey ? "ctrl+" : ""}${event.shiftKey ? "shift+" : ""}`;
  return modifiers + event.key.toLowerCase();
}

/** True on a text box, where copy and paste have to stay. */
export function takesText(target: EventTarget | null): boolean {
  return target instanceof HTMLTextAreaElement || (target instanceof HTMLInputElement && target.type !== "checkbox" && target.type !== "radio");
}

/** Keeps the window from acting like a browser page: no reload, print or page menu. */
export function behaveLikeAnApp(): void {
  document.addEventListener("contextmenu", (event) => {
    const selected = !getSelection()?.isCollapsed;
    if (!takesText(event.target) && !selected) event.preventDefault();
  });
  document.addEventListener("keydown", (event) => {
    if (BROWSER_KEYS.has(keyName(event))) event.preventDefault();
  });
}
