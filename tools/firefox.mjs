// Starts a real Firefox with a copy of the extension, for the trial scripts.
//
// Firefox is driven over WebDriver BiDi, which it ships with, on a throwaway
// profile: the person's own Firefox and its settings are not touched. Automation
// cannot open the extension's own pages there, so the copy's background asks a small
// server of this module what to do. Only the copy gets that; the extension does not.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { join } from "node:path";

const ADDON_ID = "richpresenceplus@manucabral.dev";
// Fixed through a preference, so the copy has the same address every run.
const ADDON_UUID = "6f3c2a10-5b7e-4d2a-9a51-0c4e8f7b2d19";

/** The site the control server answers on, which the copy needs permission for. */
export const CONTROL_SITE = "http://127.0.0.1/*";

const CONTROL = `
(async function control() {
  for (;;) {
    try {
      const order = await (await fetch("CONTROL_URL", { cache: "no-store" })).json();
      let result = null;
      if (order.store) await chrome.storage.local.set(order.store);
      if (order.alarm) chrome.alarms.create("try", { when: Date.now() + 100 });
      if (order.refresh) await applyRules(rules);
      if (order.status) result = await status();
      await fetch("CONTROL_URL", { method: "POST", body: JSON.stringify({ id: order.id, result }) });
    } catch {
      await new Promise((done) => setTimeout(done, 500));
    }
  }
})();
`;

const PREFERENCES = `
user_pref("browser.shell.checkDefaultBrowser", false);
user_pref("browser.aboutwelcome.enabled", false);
user_pref("browser.startup.homepage_override.mstone", "ignore");
user_pref("datareporting.policy.dataSubmissionEnabled", false);
user_pref("toolkit.telemetry.reportingpolicy.firstRun", false);
user_pref("media.autoplay.default", 0);
user_pref("media.volume_scale", "0.0");
user_pref("extensions.webextensions.uuids", ${JSON.stringify(JSON.stringify({ [ADDON_ID]: ADDON_UUID }))});
`;

function program() {
  const programs = [process.env.ProgramFiles, process.env["ProgramFiles(x86)"]];
  const candidates = programs.map((dir) => join(dir ?? "", "Mozilla Firefox", "firefox.exe"));
  const found = [process.env.FIREFOX, ...candidates, "/usr/bin/firefox"].find((path) => path && existsSync(path));
  if (!found) throw new Error("no Firefox found; set FIREFOX to its path");
  return found;
}

// A WebDriver BiDi session: one call per command, answered by id.
async function bidi(url) {
  const socket = new WebSocket(url);
  await new Promise((done, fail) => {
    socket.onopen = done;
    socket.onerror = () => fail(new Error(`cannot open ${url}`));
  });
  let next = 1;
  const waiting = new Map();
  socket.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id) waiting.get(message.id)?.(message);
  };
  const call = (method, params = {}) =>
    new Promise((done, fail) => {
      const id = next++;
      const timer = setTimeout(() => fail(new Error(`no answer to ${method}`)), 15_000);
      waiting.set(id, (message) => {
        clearTimeout(timer);
        if (message.type === "error") fail(new Error(`${method}: ${message.error} ${message.message}`));
        else done(message.result);
      });
      socket.send(JSON.stringify({ id, method, params }));
    });
  return { call, close: () => socket.close() };
}

// Hands orders to the copy's background when it asks, and takes its answers.
async function controlServer() {
  const orders = [];
  const asking = [];
  const answers = new Map();
  let last = 0;
  const flush = () => {
    while (orders.length && asking.length) {
      const response = asking.shift();
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(orders.shift()));
    }
  };
  const server = createServer((request, response) => {
    if (request.method !== "POST") {
      asking.push(response);
      flush();
      return;
    }
    let body = "";
    request.on("data", (chunk) => {
      body += chunk;
    });
    request.on("end", () => {
      const { id, result } = JSON.parse(body);
      answers.get(id)?.(result);
      answers.delete(id);
      response.end();
    });
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  const order = (what) =>
    new Promise((done, fail) => {
      const id = ++last;
      answers.set(id, done);
      orders.push({ id, ...what });
      flush();
      setTimeout(() => fail(new Error(`the extension did not take ${JSON.stringify(what)}`)), 60_000);
    });
  const close = () => {
    for (const response of asking) response.destroy();
    server.close();
  };
  return { url: `http://127.0.0.1:${server.address().port}/control`, order, close };
}

/**
 * Opens Firefox with the extension copy in `copy`, which this changes.
 *
 * The copy's manifest must allow `CONTROL_SITE`; it loses `nativeMessaging`. `order` sends `{ store, alarm,
 * refresh, status }` to the copy's background; `run` evaluates in a tab.
 *
 * @param {{ copy: string, profile: string, show?: boolean }} options
 */
export async function startFirefox({ copy, profile, show = false }) {
  const control = await controlServer();
  // Firefox lets the copy start the native host of an installed app, which answers
  // with that app's port and replaces the pairing of the trial.
  const manifestFile = join(copy, "manifest.json");
  const manifest = JSON.parse(readFileSync(manifestFile, "utf8"));
  manifest.permissions = manifest.permissions.filter((name) => name !== "nativeMessaging");
  writeFileSync(manifestFile, JSON.stringify(manifest));
  const background = join(copy, "background.js");
  writeFileSync(background, `${readFileSync(background, "utf8")}\n${CONTROL.replaceAll("CONTROL_URL", control.url)}`);
  mkdirSync(profile, { recursive: true });
  writeFileSync(join(profile, "user.js"), PREFERENCES);

  const child = spawn(
    program(),
    [...(show ? [] : ["--headless"]), "--profile", profile, "--no-remote", "--remote-debugging-port=0", "about:blank"],
    { stdio: ["ignore", "ignore", "pipe"] },
  );
  let log = "";
  const address = new Promise((done, fail) => {
    child.stderr.on("data", (chunk) => {
      log += chunk;
      const found = /WebDriver BiDi listening on (ws:\/\/\S+)/.exec(log);
      if (found) done(found[1]);
    });
    child.on("exit", () => fail(new Error(`Firefox closed: ${log.slice(-500)}`)));
    setTimeout(() => fail(new Error(`Firefox did not open its remote port: ${log.slice(-500)}`)), 30_000);
  });

  let driver = null;
  const close = async () => {
    await driver?.call("browser.close").catch(() => {});
    driver?.close();
    child.kill();
    control.close();
  };
  try {
    driver = await bidi(`${(await address).replace(/\/$/, "")}/session`);
    await driver.call("session.new", { capabilities: {} });
    const installed = await driver.call("webExtension.install", { extensionData: { type: "path", path: copy } });
    const run = async (context, expression) => {
      const answer = await driver.call("script.evaluate", {
        expression,
        target: { context },
        awaitPromise: true,
        resultOwnership: "none",
      });
      return answer.type === "exception" ? { failed: answer.exceptionDetails.text } : answer.result;
    };
    // Right after the start, Firefox can still be replacing its first window.
    const tab = async () => {
      for (let tries = 1; ; tries += 1) {
        try {
          return (await driver.call("browsingContext.create", { type: "tab" })).context;
        } catch (error) {
          if (tries === 5) throw error;
          await new Promise((done) => setTimeout(done, 500));
        }
      }
    };
    const navigate = (context, url, wait = "complete") =>
      driver.call("browsingContext.navigate", { context, url, wait });
    return { id: installed.extension, order: control.order, run, tab, navigate, close };
  } catch (error) {
    await close();
    throw error;
  }
}
