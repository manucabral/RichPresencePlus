// Runs in the page's own world, the only place that can see its variables. It runs no
// code of a presence: it answers with the data at the paths the manifest names, and the
// page script checks what it receives.
(() => {
  const MARK = Symbol.for("rpp.variables");
  const MAX_JSON = 4096;
  if (window[MARK]) return;
  Object.defineProperty(window, MARK, { value: true });

  function read(path) {
    let value = globalThis;
    for (const part of path.split(".")) {
      if (value === null || value === undefined) return null;
      value = value[part];
    }
    return typeof value === "function" ? null : value;
  }

  document.addEventListener("rpp-read", (event) => {
    const answer = {};
    try {
      for (const path of JSON.parse(event.detail)) {
        try {
          // Measured one by one: a huge object must cost its own path, not the rest.
          const text = JSON.stringify(read(path));
          answer[path] = text !== undefined && text.length <= MAX_JSON ? JSON.parse(text) : null;
        } catch {
          answer[path] = null; // A getter failed, or the object points to itself.
        }
      }
    } catch {
      return;
    }
    document.dispatchEvent(new CustomEvent("rpp-values", { detail: JSON.stringify(answer) }));
  });
})();
