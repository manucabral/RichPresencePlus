// Lets someone click the texts a new presence shows. The popup starts it on the tab
// it opened on, and it leaves the page as it was when it ends.
//
// The panel lives in a closed shadow root, so the page can neither read it nor style
// it, and it uses a constructed style sheet, which a page's rules do not block.
(() => {
  if (globalThis.__rppPicker) return;
  globalThis.__rppPicker = true;

  const { readable, selectorFor } = globalThis.RPPSelector;
  const t = (key) => chrome.i18n.getMessage(key) || key;

  const STYLE = `
    :host { all: initial; }
    .box { position: fixed; z-index: 2147483646; pointer-events: none; outline: 2px solid #19c283;
      background: rgba(25, 194, 131, 0.15); border-radius: 3px; }
    .panel { position: fixed; z-index: 2147483647; right: 16px; bottom: 16px; width: 280px; padding: 12px;
      border: 1px solid #2a2f33; border-radius: 10px; background: #0d1012; color: #e8eaec;
      font: 13px/1.4 "Segoe UI", system-ui, sans-serif; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5); }
    p { margin: 0 0 8px; }
    .picked { color: #9aa2a8; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
    .error { color: #f5a97f; }
    input { box-sizing: border-box; width: 100%; margin-bottom: 8px; padding: 6px 8px; color: #e8eaec;
      background: #050607; border: 1px solid #2a2f33; border-radius: 6px; font: inherit; }
    .row { display: flex; gap: 8px; }
    button { padding: 6px 10px; border: 1px solid #2a2f33; border-radius: 6px; background: transparent;
      color: #e8eaec; font: inherit; cursor: pointer; }
    button.main { background: #19c283; border-color: #19c283; color: #04110c; font-weight: 600; }
  `;

  const host = document.createElement("div");
  const shadow = host.attachShadow({ mode: "closed" });
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(STYLE);
  shadow.adoptedStyleSheets = [sheet];

  const box = document.createElement("div");
  box.className = "box";
  box.hidden = true;
  const panel = document.createElement("div");
  panel.className = "panel";
  panel.setAttribute("role", "dialog");
  shadow.append(box, panel);
  document.documentElement.append(host);

  const chosen = { first: null, second: null };
  let step = "first";
  let pointed = null;

  function button(label, action, main) {
    const item = document.createElement("button");
    item.type = "button";
    item.textContent = t(label);
    if (main) item.className = "main";
    item.addEventListener("click", action);
    return item;
  }

  function line(text, className) {
    const item = document.createElement("p");
    item.textContent = text;
    if (className) item.className = className;
    return item;
  }

  function preview(selector) {
    return (document.querySelector(selector)?.textContent ?? "").trim().slice(0, 80);
  }

  /** A name for the presence: the site without its `www.`, with a capital. */
  function siteName() {
    const name = location.hostname.replace(/^www\./, "").split(".")[0];
    return name.charAt(0).toUpperCase() + name.slice(1);
  }

  function end() {
    document.removeEventListener("mousemove", onMove, true);
    document.removeEventListener("click", onClick, true);
    document.removeEventListener("keydown", onKey, true);
    host.remove();
    delete globalThis.__rppPicker;
  }

  function cancel() {
    chrome.runtime.sendMessage({ type: "pick-cancel" }).catch(() => {});
    end();
  }

  async function finish(name) {
    const answer = await chrome.runtime
      .sendMessage({ type: "pick-done", name, first: chosen.first, second: chosen.second })
      .catch(() => null);
    panel.replaceChildren(line(t(answer?.ok ? "pickDone" : "pickFailed")), button("pickClose", end, true));
  }

  function draw(problem) {
    panel.replaceChildren();
    if (step === "name") {
      box.hidden = true;
      const name = document.createElement("input");
      name.value = siteName();
      name.maxLength = 64;
      name.setAttribute("aria-label", t("pickName"));
      const row = document.createElement("div");
      row.className = "row";
      const create = () => name.value.trim() && finish(name.value.trim());
      name.addEventListener("keydown", (event) => event.key === "Enter" && create());
      row.append(button("pickCreate", create, true), button("pickCancel", cancel));
      panel.append(line(t("pickName")), name, row);
      name.focus();
      return;
    }
    panel.append(line(t(step === "first" ? "pickFirst" : "pickSecond")));
    if (chosen.first) panel.append(line(preview(chosen.first), "picked"));
    if (problem) panel.append(line(t(problem), "error"));
    const row = document.createElement("div");
    row.className = "row";
    if (step === "second")
      row.append(
        button("pickSkip", () => {
          step = "name";
          draw();
        }),
      );
    row.append(button("pickCancel", cancel));
    panel.append(row);
  }

  function onMove(event) {
    if (step === "name" || event.composedPath().includes(host)) return;
    pointed = event.target;
    const area = pointed.getBoundingClientRect();
    Object.assign(box.style, {
      left: `${area.left}px`,
      top: `${area.top}px`,
      width: `${area.width}px`,
      height: `${area.height}px`,
    });
    box.hidden = false;
  }

  function onClick(event) {
    if (event.composedPath().includes(host)) return;
    // The page must not follow a link while someone is choosing a text on it.
    event.preventDefault();
    event.stopImmediatePropagation();
    if (step === "name") return;
    const element = event.target;
    const selector = readable(element) ? selectorFor(element) : null;
    if (!selector) {
      draw("pickRefused");
      return;
    }
    chosen[step] = selector;
    step = step === "first" ? "second" : "name";
    draw();
  }

  function onKey(event) {
    if (event.key === "Escape") cancel();
  }

  document.addEventListener("mousemove", onMove, true);
  document.addEventListener("click", onClick, true);
  document.addEventListener("keydown", onKey, true);
  draw();
})();
