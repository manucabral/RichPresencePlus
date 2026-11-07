// Builds a selector for an element someone clicked. It runs with the picker, and
// is tested in a real browser by tools/try-extension.mjs.
(() => {
  /** Longest selector a manifest accepts. */
  const MAX_SELECTOR = 256;
  /** Levels climbed before giving up on a shorter path. */
  const MAX_DEPTH = 8;

  // A name a person wrote, not one a build tool generated: those change each release.
  const stable = (text) => /^[A-Za-z][\w-]{1,48}$/.test(text) && !/\d{3,}/.test(text) && !/[a-z]\d[a-z]/i.test(text);

  /** Tags that hold what a person typed or that show no text. */
  const NOT_TEXT = new Set([
    "INPUT",
    "TEXTAREA",
    "SELECT",
    "OPTION",
    "BUTTON",
    "SCRIPT",
    "STYLE",
    "IMG",
    "VIDEO",
    "AUDIO",
    "SVG",
  ]);

  /**
   * False for form fields, anything editable and empty elements.
   *
   * @param {Element} element
   * @returns {boolean}
   */
  function readable(element) {
    if (NOT_TEXT.has(element.tagName.toUpperCase()) || element.isContentEditable) return false;
    if (element.closest("input, textarea, select, [contenteditable=true]")) return false;
    return (element.textContent ?? "").trim().length >= 2;
  }

  function own(element) {
    const tag = element.tagName.toLowerCase();
    if (element.id && stable(element.id)) return `#${CSS.escape(element.id)}`;
    for (const name of ["data-testid", "itemprop"]) {
      const value = element.getAttribute(name);
      if (value && stable(value)) return `${tag}[${name}="${value}"]`;
    }
    return tag;
  }

  function placed(element) {
    const tag = element.tagName.toLowerCase();
    const parent = element.parentElement;
    if (!parent) return tag;
    const same = [...parent.children].filter((child) => child.tagName === element.tagName);
    return same.length === 1 ? tag : `${tag}:nth-of-type(${same.indexOf(element) + 1})`;
  }

  /**
   * The shortest selector that finds exactly this element; null when none
   * fits a manifest.
   *
   * @param {Element} element
   * @returns {string|null}
   */
  function selectorFor(element) {
    const root = element.ownerDocument;
    const finds = (selector) => {
      const found = root.querySelectorAll(selector);
      return found.length === 1 && found[0] === element;
    };
    const alone = own(element);
    if (finds(alone)) return alone;
    const path = [own(element) === element.tagName.toLowerCase() ? placed(element) : alone];
    let current = element;
    for (let depth = 0; depth < MAX_DEPTH && current.parentElement; depth += 1) {
      current = current.parentElement;
      if (current === root.documentElement) break;
      const named = own(current);
      const anchored = named !== current.tagName.toLowerCase();
      path.unshift(anchored ? named : placed(current));
      const selector = path.join(" > ");
      if (selector.length > MAX_SELECTOR) return null;
      if (finds(selector)) return selector;
    }
    return null;
  }

  globalThis.RPPSelector = { readable, selectorFor, MAX_SELECTOR };
})();
