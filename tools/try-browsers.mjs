// Tries the automatic pairing in real browsers, with throwaway profiles.
//
//   node tools/try-browsers.mjs [chrome] [edge] [firefox]
//
// The browsers run headless and never touch the user's profile, and the
// desktop app does not need to run: the `listen` example of rpp-bridge takes
// its place on a free port, which also covers a taken preferred port. The app
// must have started once, so that the native host is registered; the host is
// the program named in that registration. Browsers inherit LOCALAPPDATA, which
// is how the host finds the pairing of this run instead of the real one.
import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const EXTENSION = resolve(ROOT, "extension");
const IDENTIFIER = "com.manucabral.richpresenceplus";
const WAIT_SECONDS = 45;

const PROGRAM_FILES = [process.env.ProgramFiles, process.env["ProgramFiles(x86)"]];

function installed(relative) {
  return PROGRAM_FILES.map((dir) => join(dir ?? "", relative)).find(existsSync);
}

// Opens a JSON-over-WebSocket link; `call` waits for the answer with its id.
async function link(url) {
  const socket = new WebSocket(url);
  await new Promise((done, fail) => {
    socket.onopen = done;
    socket.onerror = () => fail(new Error(`cannot open ${url}`));
  });
  let next = 1;
  const waiting = new Map();
  socket.onmessage = (event) => {
    const message = JSON.parse(event.data);
    waiting.get(message.id)?.(message);
  };
  return {
    call(method, params = {}) {
      const id = next++;
      const answer = new Promise((done) => waiting.set(id, done));
      socket.send(JSON.stringify({ id, method, params }));
      return answer;
    },
  };
}

const wait = (ms) => new Promise((done) => setTimeout(done, ms));
const exited = (child) => new Promise((done) => child.once("exit", done));

async function eventually(read, what) {
  for (let i = 0; i < 300; i += 1) {
    const value = read();
    if (value) return value;
    await new Promise((done) => setTimeout(done, 100));
  }
  throw new Error(`timed out waiting for ${what}`);
}

// Chrome and Edge no longer take extensions on their command line: only the
// debugging pipe can load one. The debugging port exists to close the browser.
async function chromium(exe, profile, env) {
  const child = spawn(
    exe,
    [
      "--headless=new",
      `--user-data-dir=${profile}`,
      "--remote-debugging-pipe",
      "--enable-unsafe-extension-debugging",
      "--remote-debugging-port=0",
      "--no-first-run",
      "--no-default-browser-check",
      "about:blank",
    ],
    { env, stdio: ["ignore", "ignore", "ignore", "pipe", "pipe"] },
  );
  const loaded = new Promise((done, fail) => {
    let text = "";
    child.stdio[4].on("data", (chunk) => {
      text += chunk;
      for (const line of text.split("\0").slice(0, -1)) {
        const message = JSON.parse(line);
        if (message.id === 1) message.error ? fail(new Error(message.error.message)) : done();
      }
    });
    setTimeout(() => fail(new Error("the browser did not load the extension")), 15_000);
  });
  child.stdio[3].write(
    `${JSON.stringify({ id: 1, method: "Extensions.loadUnpacked", params: { path: EXTENSION } })}\0`,
  );
  await loaded;
  const portFile = join(profile, "DevToolsActivePort");
  const text = await eventually(() => existsSync(portFile) && readFileSync(portFile, "utf8"), "the debugging port");
  const port = text.split("\n")[0].trim();
  const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  const browser = await link(version.webSocketDebuggerUrl);
  return { child, close: () => browser.call("Browser.close") };
}

// Firefox: temporary add-ons load through WebDriver BiDi.
async function firefox(exe, profile, env) {
  const child = spawn(exe, ["--headless", "--no-remote", "--profile", profile, "--remote-debugging-port", "0"], {
    env,
    stdio: ["ignore", "ignore", "pipe"],
  });
  let log = "";
  child.stderr.on("data", (chunk) => {
    log += chunk;
  });
  const found = await eventually(() => /listening on (ws:\/\/[^\s]+)/.exec(log), "the debugging port");
  const browser = await link(`${found[1]}/session`);
  await browser.call("session.new", { capabilities: {} });
  const installedAddon = await browser.call("webExtension.install", {
    extensionData: { type: "path", path: EXTENSION },
  });
  if (installedAddon.type === "error") throw new Error(installedAddon.message);
  return { child, close: () => browser.call("browser.close") };
}

const BROWSERS = {
  chrome: { start: chromium, exe: () => installed("Google/Chrome/Application/chrome.exe") },
  edge: { start: chromium, exe: () => installed("Microsoft/Edge/Application/msedge.exe") },
  firefox: { start: firefox, exe: () => installed("Mozilla Firefox/firefox.exe") },
};

// Runs one browser against one bridge; returns what the bridge saw.
async function attempt(name, listen) {
  const { start, exe } = BROWSERS[name];
  const program = exe();
  if (!program) return `${name}: not installed`;

  const home = mkdtempSync(join(tmpdir(), "rpp-try-"));
  const data = join(home, IDENTIFIER);
  const profile = join(home, "profile");
  mkdirSync(data);
  mkdirSync(profile);
  const bridge = spawn(listen, [data, String(WAIT_SECONDS)], { stdio: ["ignore", "pipe", "inherit"] });
  let output = "";
  bridge.stdout.on("data", (chunk) => {
    output += chunk;
  });
  const ended = new Promise((done) => bridge.on("exit", done));
  await eventually(() => output.includes("listening"), "the bridge");

  let running = null;
  try {
    running = await start(program, profile, { ...process.env, LOCALAPPDATA: home });
    await ended;
    const seen = /connected (.*)/.exec(output);
    return seen ? `${name}: connected ${seen[1]}` : `${name}: FAILED, no extension connected`;
  } catch (error) {
    bridge.kill();
    return `${name}: FAILED, ${error.message}`;
  } finally {
    // A closing browser may drop the link without answering.
    running?.close().catch(() => {});
    if (running) await Promise.race([exited(running.child), wait(15000)]);
    // A browser slow to exit can hold its profile; the folder is only temporary.
    try {
      rmSync(home, { recursive: true, force: true, maxRetries: 5, retryDelay: 500 });
    } catch {
      console.warn(`left behind: ${home}`);
    }
  }
}

const built = spawnSync("cargo", ["build", "-q", "-p", "rpp-bridge", "--example", "listen"], {
  cwd: ROOT,
  stdio: "inherit",
});
if (built.status !== 0) process.exit(1);
const listen = resolve(ROOT, "target/debug/examples/listen.exe");

const wanted = process.argv.slice(2);
let failed = false;
for (const name of wanted.length > 0 ? wanted : Object.keys(BROWSERS)) {
  const result = await attempt(name, listen);
  console.log(result);
  failed ||= result.includes("FAILED");
}
process.exit(failed ? 1 : 0);
