// Watches the release build for leaks: idle with the window open, a burst of moves
// between screens, then the tray.
//
//   cargo build --release -p rpp-desktop --features custom-protocol
//   node tools/try-memory.mjs [--minutes 20]
//
// It starts the application itself, so none can be running (a second start only shows the
// first). The interface is watched through the debugging port of its web view, which exists
// only for this run, and the application is closed at the end. Memory by process comes from
// the system. The heap of the page, its nodes and its listeners are read after a forced
// collection, so a number that grows is held by something and not waiting to be freed.
//
// It fails when the moves leave more behind than LIMITS allows, once the screens are warm. Idle growth is printed, not
// judged: Chromium keeps caches that settle on their own, so only a long run says more.
import { execFileSync, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const EXE = join(ROOT, "target", "release", "rpp-desktop.exe");
const SAMPLER = join(ROOT, "tools", "sample-process.ps1");
const PORT = 9444;
const ARGS = `--disable-gpu --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port=${PORT} --remote-debugging-address=127.0.0.1`;
/** What 180 moves between screens may leave behind, after a collection. */
const LIMITS = { jsMB: 1.0, nodes: 60, listeners: 60 };
const ROUTES = [
  "/",
  "/presences",
  "/presences/YouTube",
  "/manual",
  "/catalog",
  "/browser",
  "/privacy",
  "/settings",
  "/developer",
];

const minutes = Number(process.argv[process.argv.indexOf("--minutes") + 1] || 20);
const wait = (ms) => new Promise((done) => setTimeout(done, ms));

if (process.platform !== "win32") throw new Error("this tool reads Windows processes");
if (!existsSync(EXE))
  throw new Error("build the release first: cargo build --release -p rpp-desktop --features custom-protocol");
const running = execFileSync("tasklist", ["/FI", "IMAGENAME eq rpp-desktop.exe", "/NH"], { encoding: "utf8" });
if (running.includes("rpp-desktop.exe"))
  throw new Error("close the application first: a second start would only show the first");

const child = spawn(EXE, [], { env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: ARGS }, stdio: "ignore" });
const system = () =>
  JSON.parse(
    execFileSync("powershell", ["-NoProfile", "-File", SAMPLER], {
      env: { ...process.env, APP_PID: String(child.pid) },
      encoding: "utf8",
    }),
  );
const webview = (m) => +(m.renderer + m["gpu-process"] + m.utility + m.browser + m.other).toFixed(1);

async function connect() {
  for (let i = 0; i < 100; i += 1) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
      const tab = list.find((item) => item.type === "page");
      if (tab) {
        const socket = new WebSocket(tab.webSocketDebuggerUrl);
        await new Promise((done, fail) => {
          socket.onopen = done;
          socket.onerror = fail;
        });
        let next = 1;
        const waiting = new Map();
        socket.onmessage = (event) => {
          const message = JSON.parse(event.data);
          if (message.id) waiting.get(message.id)?.(message.result ?? {});
        };
        return (method, params = {}) =>
          new Promise((done, fail) => {
            const id = next++;
            const timer = setTimeout(() => fail(new Error(`no answer to ${method}`)), 15_000);
            waiting.set(id, (result) => {
              clearTimeout(timer);
              done(result);
            });
            socket.send(JSON.stringify({ id, method, params }));
          });
      }
    } catch {
      // The window is still opening.
    }
    await wait(300);
  }
  throw new Error("the window did not open");
}

let call;
async function page() {
  await call("HeapProfiler.collectGarbage");
  const { metrics } = await call("Performance.getMetrics");
  const get = (name) => metrics.find((metric) => metric.name === name)?.value ?? 0;
  return {
    jsMB: +(get("JSHeapUsedSize") / 1048576).toFixed(2),
    nodes: get("Nodes"),
    listeners: get("JSEventListeners"),
    documents: get("Documents"),
  };
}

/** Processor use since `first`, as a share of one core, for each kind of process. */
function cpuSince(first, m, seconds) {
  const share = (now, then) => (((now - then) / seconds) * 100).toFixed(2);
  return `cpu ${share(m.cpu_app, first.cpu_app)}% app, ${share(m.cpu_renderer, first.cpu_renderer)}% page, ${share(m.cpu_other, first.cpu_other)}% rest of the web view`;
}

async function report(label, first, startedAt) {
  const m = system();
  if (m.gone) throw new Error("the application closed during the run");
  const p = await page();
  const seconds = (Date.now() - startedAt) / 1000;
  const cpu = first === null ? "" : ` | ${cpuSince(first, m, seconds)}`;
  console.log(
    `${new Date().toLocaleTimeString()} ${label.padEnd(8)} app ${m.app} web view ${webview(m)} (page ${m.renderer}) | js ${p.jsMB} MB, ${p.nodes} nodes, ${p.listeners} listeners, ${p.documents} documents | ${m.handles} handles, ${m.threads} threads${cpu}`,
  );
  return { m, p };
}

const failures = [];
try {
  call = await connect();
  await call("Performance.enable");
  await call("HeapProfiler.enable");
  await wait(8000);

  console.log(`--- ${minutes} minutes idle, window open on the Panel`);
  const first = system();
  const startedAt = Date.now();
  await report("start", null, startedAt);
  for (let minute = 1; minute <= minutes; minute += 1) {
    await wait(60_000);
    if (minute % 2 === 0 || minute === minutes) await report(`${minute} min`, first, startedAt);
  }

  // A first round loads each screen's code and fills the caches of the page. A leak shows in
  // the rounds after it, so only those are judged.
  const burst = async (rounds) => {
    for (let round = 0; round < rounds; round += 1) {
      for (const route of ROUTES) {
        await call("Runtime.evaluate", { expression: `location.hash = ${JSON.stringify(`#${route}`)}` });
        await wait(60);
      }
    }
    await call("Runtime.evaluate", { expression: `location.hash = "#/"` });
    await wait(3000);
  };
  console.log("--- warming up: 2 rounds over the screens");
  const cold = await report("cold", null, Date.now());
  await burst(2);
  const before = await report("warm", null, Date.now());
  console.log(
    `warming up added: web view ${(webview(before.m) - webview(cold.m)).toFixed(1)} MB, js ${(before.p.jsMB - cold.p.jsMB).toFixed(2)} MB`,
  );

  for (const [name, rounds] of [
    ["first", 20],
    ["second", 20],
  ]) {
    console.log(`--- ${name} burst: ${rounds * ROUTES.length} moves`);
    const start = await report("before", null, Date.now());
    await burst(rounds);
    const end = await report("after", null, Date.now());
    const left = {
      jsMB: end.p.jsMB - start.p.jsMB,
      nodes: end.p.nodes - start.p.nodes,
      listeners: end.p.listeners - start.p.listeners,
    };
    console.log(
      `left behind: js ${left.jsMB.toFixed(2)} MB, ${left.nodes} nodes, ${left.listeners} listeners, web view ${(webview(end.m) - webview(start.m)).toFixed(1)} MB, documents ${end.p.documents - start.p.documents}`,
    );
    for (const key of Object.keys(LIMITS)) {
      if (left[key] > LIMITS[key])
        failures.push(`${name} burst: ${key} grew by ${left[key].toFixed(2)} (limit ${LIMITS[key]})`);
    }
  }

  console.log("--- window closed to the tray");
  const closer = `Add-Type -MemberDefinition '[DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);' -Name W -Namespace N; $p = Get-Process -Id ${child.pid}; [N.W]::PostMessage($p.MainWindowHandle, 0x10, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null`;
  execFileSync("powershell", ["-NoProfile", "-Command", closer]);
  const closedAt = Date.now();
  const start = system();
  for (const delay of [15, 15, 30, 60]) {
    await wait(delay * 1000);
    const m = system();
    const seconds = (Date.now() - closedAt) / 1000;
    console.log(
      `+${Math.round(seconds)}s`.padEnd(8),
      `app ${m.app}, ${m.app_ws} in RAM | web view ${webview(m)} | ${m.handles} handles, ${m.threads} threads | cpu ${(((m.cpu_app - start.cpu_app) / seconds) * 100).toFixed(3)}% of a core`,
    );
  }
  const rest = system();
  if (webview(rest) > 0)
    failures.push(`the web view was still there a minute and a half after closing: ${webview(rest)} MB`);
} catch (error) {
  // Say what state the window was in: a closed window is not a leak, and the person
  // watching the screen can close it.
  const state = await fetch(`http://127.0.0.1:${PORT}/json/list`)
    .then((response) => response.json())
    .then((list) => `${list.filter((item) => item.type === "page").length} pages listed`)
    .catch(() => "the debugging port is gone");
  const alive = system().gone ? "the application is gone" : "the application is running";
  failures.push(`${error.message} (${new Date().toLocaleTimeString()}; ${state}; ${alive})`);
} finally {
  child.kill();
}
console.log(failures.length ? `FAILED: ${failures.join("; ")}` : "all passed");
process.exit(failures.length ? 1 : 0);
