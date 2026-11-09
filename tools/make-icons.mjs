// Draws the pictures that `icon(name)` gives presences, as PNG files of the site.
//
//   node tools/make-icons.mjs
//
// Discord shows a small picture as a circle about 40 pixels wide over the large one,
// and takes PNG from an address but not SVG. Each picture is drawn here as SVG and
// saved at 256 x 256 by a headless Chrome, with a transparent corner around the
// circle. The names must match `ICONS` in `crates/rpp-plugins/src/prelude.js`.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const OUT = fileURLToPath(new URL("../website/public/icons/", import.meta.url));
const SIZE = 256;
const DISC = "#1d1f22";
const MARK = "#ffffff";
const line = `fill="none" stroke="${MARK}" stroke-width="18" stroke-linecap="round" stroke-linejoin="round"`;
const head = (x, y, pointing) =>
  pointing === "right"
    ? `<path d="M${x - 14} ${y - 16} L${x + 6} ${y} L${x - 14} ${y + 16} Z" fill="${MARK}" stroke="${MARK}" stroke-width="6" stroke-linejoin="round"/>`
    : `<path d="M${x + 14} ${y - 16} L${x - 6} ${y} L${x + 14} ${y + 16} Z" fill="${MARK}" stroke="${MARK}" stroke-width="6" stroke-linejoin="round"/>`;
const loop = `<path d="M76 132 V112 a26 26 0 0 1 26 -26 H168" ${line}/>${head(172, 86, "right")}
  <path d="M180 124 V144 a26 26 0 0 1 -26 26 H88" ${line}/>${head(84, 170, "left")}`;

export const ICONS = {
  play: `<path d="M106 82 L178 128 L106 174 Z" fill="${MARK}" stroke="${MARK}" stroke-width="12" stroke-linejoin="round"/>`,
  pause: `<rect x="90" y="82" width="26" height="92" rx="7" fill="${MARK}"/><rect x="140" y="82" width="26" height="92" rx="7" fill="${MARK}"/>`,
  stop: `<rect x="88" y="88" width="80" height="80" rx="12" fill="${MARK}"/>`,
  live: `<circle cx="128" cy="128" r="46" fill="#e5484d"/><circle cx="128" cy="128" r="68" fill="none" stroke="#e5484d" stroke-width="10" opacity="0.45"/>`,
  repeat: loop,
  "repeat-one": `${loop}<path d="M122 116 L132 110 V146" fill="none" stroke="${MARK}" stroke-width="10" stroke-linecap="round" stroke-linejoin="round"/>`,
  shuffle: `<path d="M70 92 H96 C136 92 124 164 164 164 H176" ${line}/>${head(180, 164, "right")}
    <path d="M70 164 H96 C136 164 124 92 164 92 H176" ${line}/>${head(180, 92, "right")}`,
};

const svg = (mark) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${SIZE}" height="${SIZE}" viewBox="0 0 256 256"><circle cx="128" cy="128" r="128" fill="${DISC}"/>${mark}</svg>`;

function chrome() {
  const programs = [process.env.ProgramFiles, process.env["ProgramFiles(x86)"], process.env.LOCALAPPDATA];
  const candidates = programs.map((dir) => join(dir ?? "", "Google", "Chrome", "Application", "chrome.exe"));
  const found = [process.env.CHROME, ...candidates, "/usr/bin/google-chrome"].find((path) => path && existsSync(path));
  if (!found) throw new Error("no Chrome found; set CHROME to its path");
  return found;
}

async function main() {
  const wait = (ms) => new Promise((done) => setTimeout(done, ms));
  const profile = mkdtempSync(join(tmpdir(), "rpp-icons-"));
  const child = spawn(
    chrome(),
    ["--headless=new", `--user-data-dir=${profile}`, "--remote-debugging-port=0", "--hide-scrollbars", "about:blank"],
    { stdio: "ignore" },
  );
  try {
    const portFile = join(profile, "DevToolsActivePort");
    while (!existsSync(portFile) || !readFileSync(portFile, "utf8").includes("\n")) await wait(100);
    const port = readFileSync(portFile, "utf8").split("\n")[0].trim();
    const tab = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" })).json();
    const socket = new WebSocket(tab.webSocketDebuggerUrl);
    await new Promise((done) => {
      socket.onopen = done;
    });
    let next = 1;
    const waiting = new Map();
    socket.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (message.id) waiting.get(message.id)?.(message.result);
    };
    const call = (method, params = {}) =>
      new Promise((done) => {
        const id = next++;
        waiting.set(id, done);
        socket.send(JSON.stringify({ id, method, params }));
      });
    await call("Emulation.setDeviceMetricsOverride", {
      width: SIZE,
      height: SIZE,
      deviceScaleFactor: 1,
      mobile: false,
    });
    await call("Emulation.setDefaultBackgroundColorOverride", { color: { r: 0, g: 0, b: 0, a: 0 } });
    mkdirSync(OUT, { recursive: true });
    const page =
      "<!doctype html><style>html,body{margin:0;background:transparent}svg{display:block}</style><body></body>";
    await call("Page.navigate", { url: `data:text/html;base64,${Buffer.from(page).toString("base64")}` });
    await wait(500);
    for (const [name, mark] of Object.entries(ICONS)) {
      // One page for all: a new navigation per picture can answer before it is drawn.
      await call("Runtime.evaluate", { expression: `document.body.innerHTML = ${JSON.stringify(svg(mark))}` });
      await wait(150);
      const { data } = await call("Page.captureScreenshot", {
        format: "png",
        clip: { x: 0, y: 0, width: SIZE, height: SIZE, scale: 1 },
      });
      writeFileSync(join(OUT, `${name}.png`), Buffer.from(data, "base64"));
      console.log(`${name}.png`);
    }
    socket.close();
  } finally {
    child.kill();
    await wait(500);
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 });
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
