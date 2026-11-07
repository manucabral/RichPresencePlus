// Tries the built interface in a headless browser, against a made-up desktop side.
//
//   pnpm --dir frontend build
//   node tools/try-interface.mjs [--shots <folder>]
//
// It answers the calls the interface makes to the desktop side with the data
// below, so it needs neither the application nor Discord, and it never reads or
// changes the settings of the person running it. The browser uses a throwaway
// profile. With --shots it saves one picture per step, to look at them.
// Besides opening every page, it fills in the forms and presses the buttons, and checks
// that each one calls the command it promises with what was typed.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const DIST = join(ROOT, "frontend", "dist");
const CALL_TIMEOUT = 10_000;
const WIDE = { width: 950, height: 650 };
const NARROW = { width: 400, height: 700 };
const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".svg": "image/svg+xml",
  ".woff2": "font/woff2",
};

// The rules of the real window, so a page that breaks them fails here too.
const CSP = JSON.parse(readFileSync(join(ROOT, "src-tauri", "tauri.conf.json"), "utf8")).app.security.csp;

const access = (more) => ({
  http: [],
  process: false,
  args: {},
  window: false,
  media: false,
  query: [],
  files: [],
  sites: [],
  ...more,
});
const presence = (name, enabled, more) => ({
  name,
  version: "1.0.0",
  description: `Shows ${name}.`,
  kind: "video",
  interval: 5,
  hide_when_paused: true,
  enabled,
  running: enabled,
  access: access(more),
  options: [],
  origin: "bundled",
  editable: false,
  author: "someone",
  color: "#19C283",
  tags: ["video"],
  icon: null,
});
const LOGO = `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="8" height="8" fill="red"/></svg>')}`;
const global = {
  paused: false,
  privacy: "full",
  excluded_domains: ["example.com"],
  excluded_applications: [],
  switch_delay_seconds: 3,
  idle_minutes: null,
  paused_minutes: 0,
  prefer_recent: false,
  quiet_programs: [],
  quiet_hours: null,
};
const STATUS = { winner: null, activity: null, rows: [], delivery_error: null, source_errors: {}, blank_sources: [] };
const OVERVIEW = {
  version: "0.0.0",
  discord_running: true,
  extensions: 1,
  browsers: ["chrome"],
  discord_user: "someone",
  paused: false,
  warning: null,
  status: STATUS,
  problems: [],
};
const ANSWERS = {
  overview: OVERVIEW,
  presences: [
    presence("YouTube", true, { sites: ["*://www.youtube.com/*"] }),
    presence("Roblox", true, {}),
    presence("Netflix", true, {}),
  ],
  presence_icons: { YouTube: LOGO },
  preferences: { global, presences: {} },
  language: { setting: "es", system: "es" },
  favorites: [],
  catalog: { available: false, items: [] },
  extension_sessions: { live: [], recent: [] },
  diagnostics: {
    version: "0.0.0",
    resources: null,
    discord_running: true,
    discord_connected: true,
    delivery_error: null,
    paused: false,
    extensions: [],
    presences: [],
    problems: [],
    report: "",
  },
  discord_client: "auto",
  pause_shortcut: null,
  author: { saved: null, discord: "someone" },
  startup: { enabled: false, hidden: false, minimize_to_tray: true, asked: true },
  log_level: "info",
  usage: { on: false, days: {} },
  local_api: { on: false, port: 28752 },
  logs: { folder: "logs", lines: [] },
  manual: { presets: [], active: null, rotation: null },
  profiles: [],
  pairing: { port: 1, token: "t", code: "c" },
  running_programs: [],
  dev_status: { watching: null, folder: null, last_reload: null, error: null },
  check_update: { configured: false, version: null, notes: null },
  window_visible: true,
  presence_versions: [],
  has_code_editor: true,
  open_in_code_editor: null,
  open_presence_folder: null,
  update_manifest: null,
  open_link: null,
  review_presence: [
    { rule: "folder", passed: true, detail: null },
    { rule: "tags", passed: false, detail: "invented" },
  ],
  prepare_presence: { path: "C:/exports/roblox", files: 3, size: 2048 },
  test_cases: [],
  inspect_presence: {
    snapshot: null,
    activity: null,
    meta: { foreground: false, paused: false, application: null, domains: [] },
    error: null,
    console: [],
  },
};

// Stands in for what Tauri puts in the page. It runs before any script.
const DESKTOP = `(() => {
  const state = { answers: ${JSON.stringify(ANSWERS)}, failing: {}, unknown: [], calls: [] };
  window.__desktop = state;
  let callbacks = 0;
  const handlers = new Map();
  const listeners = {};
  // What the desktop side does when something changes behind the window.
  state.emit = (event) => (listeners[event] ?? []).forEach((handler) => handler({ event, id: 1, payload: null }));
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
    transformCallback: (handler) => {
      callbacks += 1;
      handlers.set(callbacks, handler);
      return callbacks;
    },
    unregisterCallback: () => {},
    convertFileSrc: (path) => path,
    invoke: async (command, args) => {
      if (command === "plugin:event|listen") (listeners[args.event] ??= []).push(handlers.get(args.handler));
      if (command.startsWith("plugin:event|")) return 1;
      state.calls.push({ command, args });
      if (command in state.failing) throw state.failing[command];
      const a = state.answers;
      if (command === "save_preferences") {
        a.preferences = args.preferences;
        return args.preferences;
      }
      if (command === "set_presence_enabled") {
        a.presences = a.presences.map((p) => (p.name === args.name ? { ...p, enabled: args.enabled } : p));
        return null;
      }
      if (command === "create_program_presence") return "/presences/" + args.choice.name;
      if (command === "create_presence") return "/presences/" + args.name;
      if (command === "save_manual_preset") {
        a.manual = { ...a.manual, presets: [...a.manual.presets.filter((p) => p.name !== args.preset.name), args.preset] };
        return null;
      }
      if (command === "install_presence") {
        if (args.accepted.length > 0) return { state: "installed" };
        const access = { http: [], process: true, args: {}, window: false, media: false, query: [], files: [], sites: [] };
        return { state: "asks", access, grants: [{ kind: "process" }] };
      }
      if (command === "delete_presence") {
        a.presences = a.presences.filter((p) => p.name !== args.name);
        return null;
      }
      if (command === "set_local_api") {
        a.local_api = { ...a.local_api, on: args.on };
        return null;
      }
      if (command === "set_track_time") {
        a.usage = { ...a.usage, on: args.on };
        return null;
      }
      if (command === "clear_usage") {
        a.usage = { ...a.usage, days: {} };
        return null;
      }
      if (command === "set_language") {
        a.language = { ...a.language, setting: args.tag };
        return null;
      }
      if (command === "set_author") {
        a.author = { ...a.author, saved: args.author.trim() || null };
        return null;
      }
      if (command in state.answers) return structuredClone(state.answers[command]);
      state.unknown.push(command);
      throw "no answer for " + command;
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
})();`;

// Controls a screen reader could not announce, or a keyboard could not see.
const UNNAMED = `[...document.querySelectorAll("a, button, input, select, textarea, [role=switch], [role=tab]")]
  .filter((control) => {
    const label = control.labels?.length > 0;
    const named = control.getAttribute("aria-label") || control.getAttribute("title") || control.textContent?.trim();
    const labelled = control.getAttribute("aria-labelledby");
    const spoken = control.getAttribute("placeholder");
    return !(label || named || labelled || spoken) && control.type !== "hidden";
  })
  .map((control) => control.tagName.toLowerCase() + "." + String(control.className).slice(0, 40))`;

function browser() {
  const programs = [process.env.ProgramFiles, process.env["ProgramFiles(x86)"]];
  const candidates = [
    process.env.CHROME,
    ...programs.flatMap((dir) => [
      join(dir ?? "", "Google", "Chrome", "Application", "chrome.exe"),
      join(dir ?? "", "Microsoft", "Edge", "Application", "msedge.exe"),
    ]),
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
  ];
  const found = candidates.find((path) => path && existsSync(path));
  if (!found) throw new Error("no Chrome or Edge found; set CHROME to its path");
  return found;
}

// Serves the built interface. A path listed in `broken` answers an error.
function serve(broken) {
  const server = createServer((request, response) => {
    const path = decodeURIComponent(new URL(request.url, "http://x").pathname);
    const file = join(DIST, normalize(path === "/" ? "index.html" : path));
    if ([...broken].some((part) => path.includes(part)) || !file.startsWith(DIST) || !existsSync(file)) {
      response.writeHead(500).end();
      return;
    }
    response.writeHead(200, {
      "content-type": TYPES[extname(file)] ?? "application/octet-stream",
      "content-security-policy": CSP,
    });
    response.end(readFileSync(file));
  });
  return new Promise((done) => server.listen(0, "127.0.0.1", () => done(server)));
}

async function devtools(profile) {
  const file = join(profile, "DevToolsActivePort");
  for (let i = 0; i < 100 && !existsSync(file); i += 1) await wait(100);
  const port = readFileSync(file, "utf8").split("\n")[0];
  const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  return pages.find((page) => page.type === "page").webSocketDebuggerUrl;
}

const wait = (ms) => new Promise((done) => setTimeout(done, ms));

async function connect(url, problems) {
  const socket = new WebSocket(url);
  await new Promise((done) => (socket.onopen = done));
  let next = 1;
  const waiting = new Map();
  const requested = new Map();
  socket.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id) waiting.get(message.id)?.(message.result ?? {});
    if (message.method === "Network.requestWillBeSent")
      requested.set(message.params.requestId, message.params.request.url);
    if (message.method === "Network.loadingFailed") {
      problems.push(`${requested.get(message.params.requestId)} did not load: ${message.params.errorText}`);
    }
    if (message.method === "Runtime.exceptionThrown") {
      const details = message.params.exceptionDetails;
      problems.push(details.exception?.description ?? details.text);
    }
    if (message.method === "Runtime.consoleAPICalled" && message.params.type === "error") {
      problems.push(message.params.args.map((arg) => arg.value ?? arg.description).join(" "));
    }
  };
  // A page that stops answering must fail the run, not leave it waiting for good.
  const call = (method, params = {}) =>
    new Promise((done, fail) => {
      const id = next++;
      const timer = setTimeout(
        () => fail(new Error(`no answer to ${method}: ${params.expression ?? ""}`)),
        CALL_TIMEOUT,
      );
      waiting.set(id, (result) => {
        clearTimeout(timer);
        done(result);
      });
      socket.send(JSON.stringify({ id, method, params }));
    });
  return { call, close: () => socket.close() };
}

async function run(page, shots) {
  const failures = [];
  const check = (ok, what) => {
    console.log(`${ok ? "ok  " : "FAIL"} ${what}`);
    if (!ok) failures.push(what);
  };
  const read = async (expression) =>
    (await page.call("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true })).result?.value;
  const setSize = (size) =>
    page.call("Emulation.setDeviceMetricsOverride", { ...size, deviceScaleFactor: 1, mobile: false });
  const shot = async (name) => {
    if (!shots) return;
    // The pictures are for people; a headless browser that skips one fails nothing.
    await page
      .call("Page.captureScreenshot", { format: "png", captureBeyondViewport: false })
      .then(({ data }) => writeFileSync(join(shots, `${name}.png`), Buffer.from(data, "base64")))
      .catch(() => console.log(`no picture of ${name}`));
  };
  const waitFor = async (expression) => {
    for (let i = 0; i < 40; i += 1) {
      if (await read(expression)) return true;
      await wait(50);
    }
    return false;
  };
  const shows = (part) => waitFor(`document.querySelector("main")?.innerText.includes(${JSON.stringify(part)})`);
  // Leaving and coming back loads the page again, with the answers now in place.
  const visit = async (route) => {
    await read(`location.hash = "#/settled"`);
    await wait(40);
    await read(`location.hash = ${JSON.stringify(`#${route}`)}`);
    await wait(250);
  };
  const answer = (expression) => read(`(() => { const a = window.__desktop.answers; ${expression} })()`);

  // Every page opens, in the same place, and fits the window.
  const routes = [
    "/",
    "/presences",
    "/presences/YouTube",
    "/manual",
    "/week",
    "/catalog",
    "/browser",
    "/privacy",
    "/settings",
    "/developer",
  ];
  for (const [size, label] of [
    [WIDE, "wide"],
    [NARROW, "narrow"],
  ]) {
    await setSize(size);
    const lefts = new Set();
    for (const route of routes) {
      await visit(route);
      const seen = await waitFor(
        `document.querySelector("main h1, main [role=alert]") && !document.querySelector("main [role=alert]")`,
      );
      const fits = await read(
        `document.querySelector("main").scrollWidth <= document.querySelector("main").clientWidth`,
      );
      check(seen && fits, `${route} opens and fits (${label})`);
      if (size === WIDE) {
        const nameless = await read(UNNAMED);
        check(nameless.length === 0, `${route} names every control: ${nameless.slice(0, 4).join(", ")}`);
      }
      if (route !== "/presences/YouTube")
        lefts.add(await read(`Math.round(document.querySelector("main h1").getBoundingClientRect().left)`));
      await shot(`${label}${route.replaceAll("/", "-")}`);
    }
    check(lefts.size === 1, `titles start in the same place (${label}): ${[...lefts]}`);
  }
  await setSize(WIDE);
  const unknown = await read(`JSON.stringify([...new Set(window.__desktop.unknown)])`);
  check(unknown === "[]", `every call has an answer: ${unknown}`);

  // The keyboard reaches every control of a page, and each one shows where it is.
  const tab = async () => {
    for (const type of ["keyDown", "keyUp"]) {
      await page.call("Input.dispatchKeyEvent", { type, key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
    }
  };
  const FOCUSABLE = `(() => [...document.querySelectorAll("a[href], button, input:not([type=hidden]), select, textarea, summary, [role=switch], [role=tab]")]
    .filter((el) => {
      // A folded section keeps its controls out of the keyboard's way; its summary stays in.
      const folded = el.closest("details:not([open])");
      const hidden = folded && !(el.tagName === "SUMMARY" && el.parentElement === folded);
      // Tab stops at the chosen option of a group; the arrows move between the rest.
      const other = el.type === "radio" && !el.checked && document.querySelector(\`input[type=radio][name="\${el.name}"]:checked\`);
      return el.offsetParent !== null && !hidden && !other && !el.disabled && el.tabIndex >= 0;
    }))()`;
  for (const route of routes) {
    await visit(route);
    await waitFor(`document.querySelector("main h1, main [role=alert]")`);
    // From the first control: Tab goes on from wherever the last page left the focus.
    // The page writes down each stop as it happens: a key and a question sent from here
    // travel apart, so asking after each Tab can miss one. It ends at the last control,
    // because one more Tab would leave the page for the browser's own bar.
    const wanted = await read(`(() => {
      const all = ${FOCUSABLE};
      window.__endFocusWatch?.();
      window.__reached = new Set([all[0]]);
      window.__faint = [];
      const stop = (event) => {
        const el = event.target;
        window.__reached.add(el);
        const style = getComputedStyle(el);
        const drawn = (style.outlineStyle !== "none" && parseFloat(style.outlineWidth) > 0) || style.boxShadow !== "none";
        if (!drawn) window.__faint.push(el.tagName.toLowerCase() + ":" + (el.getAttribute("aria-label") || el.innerText || "").trim().slice(0, 24));
      };
      document.addEventListener("focusin", stop);
      window.__endFocusWatch = () => document.removeEventListener("focusin", stop);
      all[0].focus();
      return all.length;
    })()`);
    const remaining = `${FOCUSABLE}.filter((el) => !window.__reached.has(el))`;
    for (let i = 0; i < wanted + 12 && (await read(`${remaining}.length`)) > 0; i += 1) await tab();
    await wait(50);
    const faint = await read(`window.__faint`);
    const missed = await read(`${remaining}
      .map((el) => el.tagName.toLowerCase() + ":" + (el.getAttribute("aria-label") || el.innerText || "").trim().slice(0, 24))`);
    check(
      missed.length === 0,
      `${route}: the keyboard reaches all ${wanted} controls ${missed.slice(0, 4).join(", ").split(/\s+/).join(" ")}`,
    );
    check(faint.length === 0, `${route}: the focus is drawn ${[...new Set(faint)].slice(0, 4).join(", ")}`);
  }

  // The rules of the window are in force, and it does not act like a browser page.
  await visit("/");
  const blocked = await read(`new Promise((done) => {
    document.addEventListener("securitypolicyviolation", () => done(true), { once: true });
    document.head.append(Object.assign(document.createElement("style"), { textContent: "body {}" }));
    setTimeout(() => done(false), 1000);
  })`);
  check(blocked, "an inline style breaks the rules of the window");
  const stopped = (event) =>
    read(
      `(() => { const event = ${event}; document.querySelector("main").dispatchEvent(event); return event.defaultPrevented; })()`,
    );
  check(
    await stopped(`new MouseEvent("contextmenu", { bubbles: true, cancelable: true })`),
    "a right click opens no page menu",
  );
  check(
    await stopped(`new KeyboardEvent("keydown", { key: "F5", bubbles: true, cancelable: true })`),
    "F5 does not reload the window",
  );
  check(
    await stopped(`new KeyboardEvent("keydown", { key: "p", ctrlKey: true, bubbles: true, cancelable: true })`),
    "Ctrl+P does not print",
  );
  await visit("/privacy");
  await waitFor(`!!document.querySelector("main textarea")`);
  const onBox = await read(
    `(() => { const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true }); document.querySelector("main textarea").dispatchEvent(event); return event.defaultPrevented; })()`,
  );
  check(!onBox, "a text box keeps its menu, to paste");

  // The Panel says what to do when nothing shows.
  await visit("/");
  check(await shows("Abrí YouTube, Roblox o Netflix para que aparezca acá."), "the Panel names what to open");
  await shot("panel-open");
  await answer(`a.presences = a.presences.map((p) => ({ ...p, access: { ...p.access, media: true } }));`);
  await visit("/");
  check(await shows("Reproducí algo para que aparezca acá."), "the Panel asks to play something");
  await answer(`a.presences = a.presences.map((p) => ({ ...p, enabled: false }));`);
  await visit("/");
  check(await shows("Ninguna presencia está activa."), "the Panel says nothing is on");
  check(await read(`!!document.querySelector('main a[href="#/presences"]')`), "the Panel links to the presences");
  await shot("panel-none");
  await answer(`a.overview = { ...a.overview, paused: true };`);
  await visit("/");
  check(await shows("Ninguna presencia ofrece una actividad."), "a paused Panel keeps its usual text");
  await answer(`a.overview = { ...a.overview, paused: false };`);

  // An activity with no image takes the logo of its presence.
  const activity = {
    details: "Navegando en YouTube",
    state: null,
    kind: "watching",
    start: null,
    end: null,
    large_image: null,
    large_text: null,
    small_image: null,
    small_text: null,
    buttons: [],
  };
  const row = {
    source: "YouTube",
    label: "YouTube",
    priority: 60,
    kind: "video",
    selected: true,
    reason: "Selected: Video, priority 60",
  };
  await answer(`a.presences = a.presences.map((p) => ({ ...p, enabled: true }));
    a.overview = { ...a.overview, status: { ...a.overview.status, winner: "YouTube", activity: ${JSON.stringify(activity)}, rows: [${JSON.stringify(row)}] } };`);
  await visit("/");
  await shows("Navegando en YouTube");
  const logoShown = await waitFor(
    `[...document.querySelectorAll("main img")].some((img) => img.classList.contains("object-contain") && img.src.startsWith("data:"))`,
  );
  check(logoShown, "an activity with no image shows the logo");
  await shot("panel-logo");
  await answer(
    `a.overview = { ...a.overview, status: { ...a.overview.status, winner: null, activity: null, rows: [] } };`,
  );

  // A presence the privacy rules leave with nothing says why it will not publish.
  await answer(
    `a.preferences = { ...a.preferences, global: { ...a.preferences.global, excluded_domains: ["www.youtube.com"] } };`,
  );
  await visit("/presences");
  check(await shows("excluida"), "the list marks an excluded presence");
  await visit("/presences/YouTube");
  check(await shows("No va a publicar"), "an excluded presence says why it will not publish");
  check(await read(`!!document.querySelector('main a[href="#/privacy"]')`), "it links to Privacy");
  await shot("presence-excluded");
  await answer(`a.preferences = { ...a.preferences, global: ${JSON.stringify(global)} };`);

  // The Developer page opens on what the person can do, one tab at a time.
  const developerTab = () => read(`document.querySelector('main [role=tab][aria-selected=true]')?.innerText`);
  await visit("/developer");
  await shows("Crear una presencia");
  check((await developerTab()) === "Crear", "with no presence of its own, Developer opens on Create");
  await answer(`a.presences = a.presences.map((p, i) => ({ ...p, editable: i === 0 }));`);
  await visit("/developer");
  await shows("Abrir carpeta");
  check((await developerTab()) === "Programar", "with a presence of its own, Developer opens on Code");
  await shot("developer-code");
  for (const [tab, part] of [
    ["Probar", "Vigilar"],
    ["Publicar", "Publicar en el catálogo"],
    ["Crear", "Crear una presencia"],
  ]) {
    await read(
      `[...document.querySelectorAll('main [role=tab]')].find((item) => item.innerText === ${JSON.stringify(tab)}).click()`,
    );
    await wait(150);
    const opened =
      (await developerTab()) === tab &&
      (await shows(part)) &&
      !(await read(`!!document.querySelector("main [role=alert]")`));
    check(opened, `the ${tab} tab opens`);
    await shot(`developer-${tab.toLowerCase()}`);
  }
  await answer(`a.presences = a.presences.map((p) => ({ ...p, editable: false }));`);

  // Privacy offers to save only after a change, and a failure says so.
  await visit("/privacy");
  await shows("Pausar publicación");
  const saveShown = `[...document.querySelectorAll("main button")].some((button) => button.innerText === "Guardar")`;
  check(!(await read(saveShown)), "nothing to save at first");
  await read(`document.querySelector('main [role="switch"]').click()`);
  check(await waitFor(saveShown), "a change brings the save bar");
  await shot("privacy-changed");
  await read(`window.__desktop.failing.save_preferences = "the disk is full"`);
  await read(`[...document.querySelectorAll("main button")].find((button) => button.innerText === "Guardar").click()`);
  const toast = `document.querySelector("ol[aria-live]")?.innerText ?? ""`;
  check(await waitFor(`(${toast}).includes("No se pudo guardar")`), "a failure shows a notice");
  check((await read(toast)).includes("the disk is full"), "the notice gives the reason");
  await shot("notice");
  await read(`delete window.__desktop.failing.save_preferences`);

  // Each control reaches the command it promises, with what the person typed.
  // A field is found by its aria-label, or by the label that wraps it.
  const control = (label) => `(() => {
    const named = document.querySelector(${JSON.stringify(`main [aria-label=${JSON.stringify(label)}]`)});
    if (named) return named;
    const wrapper = [...document.querySelectorAll("main label")].find((item) => item.firstElementChild?.textContent === ${JSON.stringify(label)});
    return wrapper?.querySelector("input, select, textarea") ?? null;
  })()`;
  const hasField = (label) => waitFor(`!!${control(label)}`);
  const type = (label, value) =>
    read(`(() => {
      const el = ${control(label)};
      if (!el) return false;
      el.value = ${JSON.stringify(value)};
      el.dispatchEvent(new Event(el.tagName === "SELECT" ? "change" : "input", { bubbles: true }));
      return true;
    })()`);
  // The button called `name` beside a field, or in the page when no field is given.
  const press = (name, beside) =>
    read(`(() => {
      const scope = ${beside ? `${control(beside)}?.parentElement` : "document.querySelector('main')"};
      const button = [...(scope?.querySelectorAll("button:not([role=tab])") ?? [])].find((item) => item.innerText.trim() === ${JSON.stringify(name)});
      if (!button || button.disabled) return false;
      button.click();
      return true;
    })()`);
  // Like `press`, for a button that is busy with the last click and frees itself.
  const pressSoon = async (name, beside) => {
    for (let i = 0; i < 40; i += 1) {
      if (await press(name, beside)) return true;
      await wait(50);
    }
    return false;
  };
  const called = async (command) => {
    const found = await waitFor(`window.__desktop.calls.some((call) => call.command === ${JSON.stringify(command)})`);
    if (!found) return null;
    return JSON.parse(
      await read(
        `JSON.stringify(window.__desktop.calls.filter((call) => call.command === ${JSON.stringify(command)}).at(-1).args)`,
      ),
    );
  };
  const forget = () => read(`window.__desktop.calls.length = 0`);
  const same = (value, expected) => JSON.stringify(value) === JSON.stringify(expected);

  // The switch of a presence turns it off, and the list shows it.
  await forget();
  await visit("/presences");
  const toggle = `document.querySelector('main [role=switch][aria-label="Activar o desactivar Roblox"]')`;
  check(await waitFor(`!!${toggle}`), "the list has a switch for each presence");
  await read(`${toggle}.click()`);
  const turned = await called("set_presence_enabled");
  check(same(turned, { name: "Roblox", enabled: false }), `the switch turns Roblox off: ${JSON.stringify(turned)}`);
  check(await waitFor(`${toggle}?.getAttribute("aria-checked") === "false"`), "the list shows it off");

  // A running program becomes a presence with the two lines that were typed.
  await forget();
  await answer(`a.running_programs = ["Code.exe", "my-editor.exe"];`);
  await visit("/presences");
  check(await waitFor(`!!document.querySelector("main details summary")`), "the list offers to add a program");
  await read(
    `[...document.querySelectorAll("main details summary")].find((item) => item.innerText === "Agregar un programa").click()`,
  );
  check(await hasField("Programa"), "the program picker opens");
  check(!(await press("Crear presencia")), "a program with no first line cannot be created");
  await type("Programa", "my-editor.exe");
  await type("Primera línea, como Escribiendo código", "Escribiendo código");
  await type("Segunda línea (opcional)", "En casa");
  check(await pressSoon("Crear presencia"), "the create button is ready");
  const made = await called("create_program_presence");
  check(
    same(made, {
      choice: { name: "My Editor", program: "my-editor.exe", details: "Escribiendo código", state: "En casa" },
    }),
    `a program becomes a presence: ${JSON.stringify(made)}`,
  );
  check(await waitFor(`(${toast}).includes("My Editor está lista")`), "the notice says it is ready");
  await shot("program-created");

  // The excluded sites go to the host as the lines that were typed.
  await forget();
  await visit("/privacy");
  check(await hasField("Sitios excluidos"), "Privacy has the box of excluded sites");
  await type("Sitios excluidos", "example.com\n  foo.org  \n\n");
  check(await waitFor(saveShown), "typing a site brings the save bar");
  check(await press("Guardar"), "the save button works");
  const kept = await called("save_preferences");
  check(
    same(kept?.preferences.global.excluded_domains, ["example.com", "foo.org"]),
    `the sites are saved by line: ${JSON.stringify(kept?.preferences.global.excluded_domains)}`,
  );
  check(await waitFor(`!(${saveShown})`), "the save bar goes once it is saved");

  // The automatic rules go to the host: the programs by line, the hours in minutes.
  await forget();
  await visit("/privacy");
  check(await hasField("Mientras corren estos programas"), "Privacy has the automatic rules");
  await type("Mientras corren estos programas", "obs64.exe\nzoom.exe");
  await read(`document.querySelector('main [role=switch][aria-label="En ciertas horas"]').click()`);
  check(await hasField("Desde"), "turning the hours on shows the times");
  await type("hasta", "06:30");
  check((await waitFor(saveShown)) && (await press("Guardar")), "the rules can be saved");
  const rules = (await called("save_preferences"))?.preferences.global;
  check(
    same(rules?.quiet_programs, ["obs64.exe", "zoom.exe"]),
    `the programs are saved by line: ${JSON.stringify(rules?.quiet_programs)}`,
  );
  check(
    same(rules?.quiet_hours, { from: 22 * 60, to: 6 * 60 + 30 }),
    `the hours are saved in minutes: ${JSON.stringify(rules?.quiet_hours)}`,
  );
  await shot("privacy-rules");
  await answer(`a.preferences = { ...a.preferences, global: ${JSON.stringify(global)} };`);

  // A status of the Manual page is saved with its name, its application and its first line.
  await forget();
  await visit("/manual");
  check(await hasField("Nombre"), "Manual has a form");
  await type("Nombre", "Mi estado");
  await type("ID de la aplicación", "123456789012345678");
  await type("Primera línea", "Hola");
  check(await press("Guardar"), "the status can be saved");
  const status = await called("save_manual_preset");
  check(
    status?.preset.name === "Mi estado" &&
      status.preset.client_id === "123456789012345678" &&
      status.preset.activity.details === "Hola",
    `a status is saved as typed: ${String(JSON.stringify(status?.preset)).slice(0, 120)}`,
  );
  check(
    await waitFor(
      `document.querySelector("main")?.innerText.includes("Estados guardados") && document.querySelector("main").innerText.split("Mi estado").length > 2`,
    ),
    "the saved status joins the list",
  );
  await shot("manual-saved");

  // Settings: the author goes to the host, and the language changes the words.
  await forget();
  await visit("/settings");
  check(await hasField("Autor de tus presencias"), "Settings has the author box");
  // The box fills in once the host answers; text typed before that would be replaced.
  check(
    await waitFor(`${control("Autor de tus presencias")}?.placeholder === "someone"`),
    "the author box shows the Discord name",
  );
  await type("Autor de tus presencias", "Manu");
  check(await press("Guardar", "Autor de tus presencias"), "the author can be saved");
  const author = await called("set_author");
  check(same(author, { author: "Manu" }), `the author is saved: ${JSON.stringify(author)}`);
  check(await waitFor(`(${toast}).includes("Autor guardado")`), "the notice says the author is saved");
  await type("Idioma", "en");
  const language = await called("set_language");
  check(same(language, { tag: "en" }), `the language is saved: ${JSON.stringify(language)}`);
  check(await shows("Language"), "the page changes to the new language");
  await shot("settings-english");
  await type("Language", "auto");
  check(await shows("Idioma"), "automatic goes back to the language of the system");

  // Developer: Create sends the name and the template, and a refusal says why.
  await forget();
  await visit("/developer");
  check(await hasField("Nombre"), "Developer opens on Create");
  await type("Nombre", "Mi presencia");
  check(await press("Crear", "Nombre"), "the create button works");
  const created = await called("create_presence");
  check(
    same(created, { name: "Mi presencia", template: "web" }),
    `a presence is created from a template: ${JSON.stringify(created)}`,
  );
  // A new presence moves the page to the live view; Create is one tab away.
  await read(`[...document.querySelectorAll('main [role=tab]')].find((item) => item.innerText === "Crear").click()`);
  check(await hasField("Nombre"), "Create is still one tab away");
  await read(`window.__desktop.failing.create_presence = "already exists"`);
  await type("Nombre", "Otra");
  check(await pressSoon("Crear", "Nombre"), "the create button frees itself after the last click");
  check(await waitFor(`(${toast}).includes("already exists")`), "a refusal to create says why");
  await read(`delete window.__desktop.failing.create_presence`);

  // Deleting a presence of one's own takes two clicks, and only names that one.
  await forget();
  await answer(`a.presences = a.presences.map((p) => ({ ...p, editable: p.name === "Roblox" }));`);
  await visit("/developer");
  check(await shows("Eliminar presencia"), "Code offers to delete a presence of one's own");
  check(await press("Eliminar presencia"), "the first click asks");
  check(await shows("Sí, eliminar Roblox"), "the question names the presence");
  check(
    !(await read(`window.__desktop.calls.some((call) => call.command === "delete_presence")`)),
    "one click deletes nothing",
  );
  check(await press("Conservar"), "the person can keep it");
  check(await pressSoon("Eliminar presencia"), "the question can come back");
  check(await pressSoon("Sí, eliminar Roblox"), "the second click deletes");
  check(same(await called("delete_presence"), { name: "Roblox" }), "only that presence is deleted");
  check(await waitFor(`(${toast}).includes("Roblox se fue a la Papelera")`), "the notice says where it went");
  await answer(`a.presences = a.presences.map((p) => ({ ...p, editable: false }));`);

  // Code opens the editor when there is one, and the folder always.
  await forget();
  await answer(`a.presences = a.presences.map((p) => ({ ...p, editable: p.name === "YouTube" }));`);
  await visit("/developer");
  check(await press("Abrir en VS Code"), "the editor button is there");
  check(same(await called("open_in_code_editor"), { name: "YouTube" }), "the editor opens that presence");
  check(await press("Abrir carpeta"), "the folder button is there");
  check(same(await called("open_presence_folder"), { name: "YouTube" }), "the file manager opens that presence");
  await answer(`a.has_code_editor = false;`);
  await visit("/");
  await visit("/developer");
  check(await shows("Visual Studio Code explica"), "without the editor the page says what it gives");
  check(
    !(await read(`document.querySelector("main").innerText.includes("Abrir en VS Code")`)),
    "without the editor its button goes away",
  );
  await answer(`a.has_code_editor = true;`);

  // Tags are chosen from the list and saved as a list.
  await forget();
  const tagButton = (label) =>
    `[...document.querySelectorAll('main fieldset button')].find((item) => item.innerText === ${JSON.stringify(label)})`;
  check(await waitFor(`!!${tagButton("Música")}`), "the tags of the list are offered");
  check(
    !(await read(
      `[...document.querySelectorAll("main input")].some((item) => /etiquetas/i.test(item.placeholder + item.ariaLabel))`,
    )),
    "tags cannot be typed",
  );
  await read(`${tagButton("Música")}.click()`);
  check(await waitFor(`${tagButton("Música")}.getAttribute("aria-pressed") === "true"`), "a tag turns on");
  check(await press("Guardar datos"), "the details can be saved");
  check(
    (await called("update_manifest"))?.changes?.tags?.includes("music"),
    "the saved tag is its word, not its label",
  );
  await answer(
    `a.presences = a.presences.map((p) => ({ ...p, tags: ["video", "streaming", "movie", "series", "anime", "live", "sport", "music"] }));`,
  );
  await visit("/");
  await visit("/developer");
  check(await waitFor(`${tagButton("Radio")}?.disabled === true`), "the ninth tag waits");

  // Publish: a failed rule holds the steps back, and each step frees the next.
  await forget();
  await read(`[...document.querySelectorAll('main [role=tab]')].find((item) => item.innerText === "Publicar").click()`);
  check(await shows("Tiene etiquetas de la lista"), "the rules are shown by name");
  check(await shows("invented"), "a failed rule says what is wrong");
  const button = (label) =>
    `[...document.querySelectorAll("main button")].find((item) => item.innerText === ${JSON.stringify(label)})`;
  check(await read(`${button("Preparar carpeta")}.disabled`), "a failed rule stops preparing");
  check(await read(`${button("Abrir GitHub")}.disabled`), "there is nothing to upload yet");
  await shot("developer-publish");
  await answer(`a.review_presence = [{ rule: "tags", passed: true, detail: null }];`);
  check(await press("Comprobar de nuevo"), "the check can run again");
  check(await waitFor(`!${button("Preparar carpeta")}.disabled`), "passing frees the next step");
  check(await press("Preparar carpeta"), "the folder can be prepared");
  check(same(await called("prepare_presence"), { name: "YouTube" }), "only that presence is prepared");
  check(await shows("Carpeta lista: 3 archivos"), "the page says what it made");
  check(await waitFor(`!${button("Abrir GitHub")}.disabled`), "a prepared folder frees the upload");
  check(await press("Abrir GitHub"), "the upload page can be opened");
  check(same(await called("open_link"), { target: "upload" }), "the upload opens by name, never by address");
  await answer(`a.presences = a.presences.map((p) => ({ ...p, editable: false, tags: [] }));`);

  // A folder deleted in the file manager leaves Developer without a reload.
  await answer(`a.presences = a.presences.map((p) => ({ ...p, editable: true }));
    a.dev_status = { watching: "Netflix", folder: "C:/presences/netflix", last_reload: null, error: null };`);
  await visit("/developer");
  await read(`[...document.querySelectorAll('main [role=tab]')].find((item) => item.innerText === "Probar").click()`);
  const picker = `document.querySelector('main select[aria-label="Elegí una presencia"], main select')`;
  check(
    await waitFor(`${picker}?.disabled === true && ${picker}.value === "Netflix"`),
    "a watched presence holds the picker",
  );
  await answer(`a.presences = a.presences.filter((p) => p.name !== "Netflix");
    a.dev_status = { watching: null, folder: null, last_reload: null, error: null };`);
  await read(`window.__desktop.emit("app-changed")`);
  check(await waitFor(`${picker}?.disabled === false`), "the picker comes back when the folder is gone");
  check(
    !(await read(`[...${picker}.options].some((option) => option.value === "Netflix")`)),
    "the deleted presence leaves the list",
  );
  await answer(`a.presences = a.presences.map((p) => ({ ...p, editable: false }));`);

  // The local address stays out of sight until it is turned on.
  await forget();
  await visit("/settings");
  const apiSwitch = `document.querySelector('main [role=switch][aria-label="Overlays y Stream Deck"]')`;
  check(await waitFor(`!!${apiSwitch}`), "Settings offers the local address");
  check(!(await read(`document.querySelector("main").innerText.includes("/overlay")`)), "off, it gives no address");
  await read(`${apiSwitch}.click()`);
  check(same(await called("set_local_api"), { on: true }), "the switch turns it on");
  check(await shows("http://127.0.0.1:28752/overlay"), "on, it gives the address");
  await answer(`a.local_api = { on: false, port: 28752 };`);

  // Your week: off until the person turns it on, then the time, then gone.
  await forget();
  await visit("/week");
  check(await shows("Contar el tiempo"), "Your week opens with its switch");
  check(
    !(await read(`document.querySelector("main").innerText.includes("Esta semana")`)),
    "with nothing counted there is no chart",
  );
  await read(`document.querySelector('main [role=switch]').click()`);
  check(same(await called("set_track_time"), { on: true }), "the switch turns the count on");
  const key = (back) => {
    const date = new Date();
    date.setDate(date.getDate() - back);
    return [
      date.getFullYear(),
      String(date.getMonth() + 1).padStart(2, "0"),
      String(date.getDate()).padStart(2, "0"),
    ].join("-");
  };
  const counted = { [key(0)]: { YouTube: 5400, Roblox: 600 }, [key(2)]: { YouTube: 120 }, [key(9)]: { Viejo: 9000 } };
  await answer(`a.usage = { on: true, days: ${JSON.stringify(counted)} };`);
  await visit("/week");
  check(await shows("Esta semana: 1 h 42 min"), "the week adds up its seven days");
  check(await shows("1 h 32 min"), "each presence shows its time");
  check(
    !(await read(`document.querySelector("main").innerText.includes("Viejo")`)),
    "what is older than a week stays out",
  );
  const bars = await read(`[...document.querySelectorAll('main button[aria-label*=":"]')].length`);
  check(bars === 7, `the chart has seven days: ${bars}`);
  await shot("week");
  check(await press("Borrar todo"), "the time can be deleted");
  check(await pressSoon("Sí, borrar todo"), "deleting asks once more");
  await called("clear_usage");
  check(await shows("Todavía no hay tiempo contado"), "after deleting the page is empty");
  await answer(`a.usage = { on: false, days: {} };`);

  // An update that asks for more waits for a yes, and the yes names what was shown.
  await forget();
  const update = {
    name: "Alfa",
    version: "1.1.0",
    description: "Shows Alfa.",
    kind: "game",
    size: 1000,
    state: "update",
    installed_version: "1.0.0",
    history: [],
    author: null,
    color: null,
    tags: [],
    icon: null,
  };
  await answer(`a.catalog = { available: true, items: [${JSON.stringify(update)}] };`);
  await visit("/catalog/Alfa");
  check(await shows("Actualización disponible"), "the catalog offers the update");
  check(await pressSoon("Actualizar"), "the update can be started");
  check(await shows("Esta actualización pide más acceso"), "an update that asks for more stops and says so");
  check(await shows("programas"), "it says what the update adds");
  await shot("catalog-asks");
  check(await press("Ahora no"), "the person can say not now");
  check(
    await waitFor(`!document.querySelector("main").innerText.includes("pide más acceso")`),
    "not now closes the question",
  );
  check(await pressSoon("Actualizar"), "the update can be started again");
  await shows("Esta actualización pide más acceso");
  check(await pressSoon("Aceptar y actualizar"), "the person can accept");
  await waitFor(`window.__desktop.calls.filter((call) => call.command === "install_presence").length === 3`);
  const yes = await called("install_presence");
  check(
    same(yes, { name: "Alfa", accepted: [{ kind: "process" }] }),
    `the yes names what was shown: ${JSON.stringify(yes)}`,
  );
  check(
    await waitFor(`!document.querySelector("main").innerText.includes("pide más acceso")`),
    "the question closes once accepted",
  );
  await answer(`a.catalog = { available: false, items: [] };`);

  const left = await read(`JSON.stringify([...new Set(window.__desktop.unknown)])`);
  check(left === "[]", `no flow asked for a command without an answer: ${left}`);

  return { failures, read, visit, shows, shot };
}

// A page whose code does not load: the menu stays and the page says so.
async function failedPage({ page, check, broken, shots, base }) {
  broken.add("DeveloperPage");
  // Only a new page: this one already holds the code it loaded earlier.
  const read = async (expression) =>
    (await page.call("Runtime.evaluate", { expression, returnByValue: true })).result?.value;
  // Another address, so the browser loads a new page instead of keeping this one.
  await page.call("Page.navigate", { url: `${base}/?fresh=1#/` });
  for (
    let i = 0;
    i < 100 && !(await read(`location.search === "?fresh=1" && !!document.querySelector("nav a")`));
    i += 1
  )
    await wait(5);
  // Right after the first drawing, before the page listens for moves.
  await read(`location.hash = "#/settings"`);
  let moved = false;
  for (let i = 0; i < 100 && !moved; i += 1) {
    moved = await read(`document.querySelector("main h1")?.innerText === "Ajustes"`);
    await wait(50);
  }
  check(moved, "a move during the start is not lost");
  await read(`location.hash = "#/developer"`);
  let alert = false;
  for (let i = 0; i < 120 && !alert; i += 1) {
    alert = await read(`!!document.querySelector("main [role=alert]")`);
    await wait(50);
  }
  check(alert, "a page that fails to load explains itself");
  check(await read(`document.querySelector("nav a") !== null`), "the menu stays after a failure");
  if (shots) {
    const { data } = await page.call("Page.captureScreenshot", { format: "png" });
    writeFileSync(join(shots, "failed.png"), Buffer.from(data, "base64"));
  }
  await read(`location.hash = "#/presences"`);
  let recovered = false;
  for (let i = 0; i < 60 && !recovered; i += 1) {
    recovered = await read(`!document.querySelector("main [role=alert]") && !!document.querySelector("main h1")`);
    await wait(50);
  }
  if (!recovered)
    console.log(await read(`location.hash + " | " + document.querySelector("main").innerText.slice(0, 200)`));
  check(recovered, "going to another page recovers");
  broken.clear();
}

async function main() {
  if (!existsSync(join(DIST, "index.html"))) throw new Error("build the interface first: pnpm --dir frontend build");
  const flag = process.argv.indexOf("--shots");
  const shots = flag >= 0 ? process.argv[flag + 1] : null;
  if (shots) mkdirSync(shots, { recursive: true });

  const broken = new Set();
  const server = await serve(broken);
  const base = `http://127.0.0.1:${server.address().port}`;
  const profile = mkdtempSync(join(tmpdir(), "rpp-interface-"));
  const sandbox = process.platform === "linux" ? ["--no-sandbox"] : [];
  const child = spawn(
    browser(),
    [
      "--headless=new",
      `--user-data-dir=${profile}`,
      "--remote-debugging-port=0",
      "--no-first-run",
      "--no-default-browser-check",
      ...sandbox,
      "about:blank",
    ],
    { stdio: "ignore" },
  );

  let failures = [];
  try {
    const problems = [];
    const page = await connect(await devtools(profile), problems);
    await page.call("Page.enable");
    await page.call("Runtime.enable");
    // A page kept by the browser would hide a file that stopped loading.
    await page.call("Network.enable");
    await page.call("Network.setCacheDisabled", { cacheDisabled: true });
    // A window with no one using it must still behave as the one in front.
    await page.call("Emulation.setFocusEmulationEnabled", { enabled: true });
    await page.call("Page.addScriptToEvaluateOnNewDocument", { source: DESKTOP });
    await page.call("Emulation.setDeviceMetricsOverride", { ...WIDE, deviceScaleFactor: 1, mobile: false });
    await page.call("Page.navigate", { url: `${base}/` });
    await wait(800);

    const result = await run(page, shots);
    failures = result.failures;
    const check = (ok, what) => {
      console.log(`${ok ? "ok  " : "FAIL"} ${what}`);
      if (!ok) failures.push(what);
    };
    // Calls that fail on purpose report to the console; those are not faults.
    const faults = problems.filter((problem) => !/the disk is full/.test(problem));
    check(faults.length === 0, `no errors in the console: ${faults.slice(0, 3).join(" | ")}`);

    await failedPage({ page, check, broken, shots, base });
    page.close();
  } finally {
    child.kill();
    server.close();
    await wait(500);
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  }
  console.log(failures.length ? `${failures.length} failed` : "all passed");
  process.exitCode = failures.length ? 1 : 0;
}

await main();
