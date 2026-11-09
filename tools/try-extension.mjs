// Tries the rules of the extension in a real browser, against a bridge of its own.
//
//   node tools/try-extension.mjs [--show] [--unlimited]
//
// A copy of the extension runs with permission for 127.0.0.1, as if the user had
// given it to another presence, so no permission prompt appears. The browser uses
// a throwaway profile; the app and the settings of the person running this are not
// touched. It is headless unless --show asks for a window, with a pause per step
// and each page naming its step. --unlimited takes the size check out of the copy,
// to see what it prevents.
import { spawn, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const SITE = "http://127.0.0.1/*";
// Fixed by `key` in the manifest; the same as CHROMIUM_EXTENSIONS in rpp-bridge.
const EXTENSION_ID = "iijcphjgahgdkdeaibcgagjfoaedajjk";
const wait = (ms) => new Promise((done) => setTimeout(done, ms));

function browser() {
  const programs = [process.env.ProgramFiles, process.env["ProgramFiles(x86)"]];
  const candidates = programs.flatMap((dir) => [
    join(dir ?? "", "Google", "Chrome", "Application", "chrome.exe"),
    join(dir ?? "", "Microsoft", "Edge", "Application", "msedge.exe"),
  ]);
  const found = [process.env.CHROME, ...candidates, "/usr/bin/google-chrome"].find((path) => path && existsSync(path));
  if (!found) throw new Error("no Chrome or Edge found; set CHROME to its path");
  return found;
}

async function connect(url) {
  const socket = new WebSocket(url);
  await new Promise((done, fail) => {
    socket.onopen = done;
    socket.onerror = () => fail(new Error(`cannot open ${url}`));
  });
  let next = 1;
  const waiting = new Map();
  socket.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id) waiting.get(message.id)?.(message.result ?? { failed: message.error });
  };
  const call = (method, params = {}) =>
    new Promise((done, fail) => {
      const id = next++;
      const timer = setTimeout(() => fail(new Error(`no answer to ${method}`)), 10_000);
      waiting.set(id, (result) => {
        clearTimeout(timer);
        done(result);
      });
      socket.send(JSON.stringify({ id, method, params }));
    });
  return { call, close: () => socket.close() };
}

async function main() {
  const unlimited = process.argv.includes("--unlimited");
  const show = process.argv.includes("--show");
  // A folder for the picture of the popup the stores ask for.
  const storeFlag = process.argv.indexOf("--store");
  const storeFolder = storeFlag >= 0 ? process.argv[storeFlag + 1] : null;
  const built = spawnSync("cargo", ["build", "-q", "-p", "rpp-bridge", "--example", "pages"], {
    cwd: ROOT,
    stdio: "inherit",
  });
  if (built.status !== 0) process.exit(1);

  const home = mkdtempSync(join(tmpdir(), "rpp-extension-"));
  const copy = join(home, "extension");
  cpSync(join(ROOT, "extension"), copy, { recursive: true, filter: (path) => !path.includes("node_modules") });
  const manifest = JSON.parse(readFileSync(join(copy, "manifest.json"), "utf8"));
  // Wider than the sites chosen, as when a presence for every subdomain asked first.
  manifest.host_permissions = [SITE.replace("http:", "*:"), "http://localhost/*"];
  writeFileSync(join(copy, "manifest.json"), JSON.stringify(manifest));
  if (unlimited) {
    const worker = join(copy, "background.js");
    writeFileSync(worker, readFileSync(worker, "utf8").replace("if (fitsLink(page)) {", "if (true) {"));
  }
  if (storeFolder) {
    // The picture opens the popup in a tab, which the throwaway copy has to allow.
    const rules = join(copy, "messages.js");
    writeFileSync(rules, readFileSync(rules, "utf8").replace("return !sender.tab && typeof", "return typeof"));
  }
  // The popup's click is the only way in, and automation cannot press it.
  const background = join(copy, "background.js");
  writeFileSync(background, `${readFileSync(background, "utf8")}\nglobalThis.startPick = startPick;\n`);

  const server = createServer((request, response) => {
    if (request.url.startsWith("/api/")) {
      // Answers the page asks for: one before the extension is ready, one after.
      response.writeHead(200, { "content-type": "application/json" });
      response.end(
        request.url === "/api/early"
          ? '{"data":{"title":"From an answer","secret":"stays"}}'
          : '{"now":{"track":"From XHR"}}',
      );
      return;
    }
    if (request.url.startsWith("/framed")) {
      response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      response.end(
        `<!doctype html><title>Framed</title>
          <script>fetch("/api/early").then((r) => r.json()).then((d) => { document.getElementById("early").textContent = d.data.title; });</script>
          <p id="early"></p><ul><li class="tag">Action</li><li class="tag">Drama</li></ul>
          <script id="data" type="application/json">{"video":{"title":"From JSON"}}</script>
          <div id="poster" style="color: rgb(1, 2, 3)"></div><x-player id="host"></x-player>
          <script>
            window.app = { video: { episode: 12, cast: ["A", "B"], play() {} } };
            localStorage.setItem("player", '{"track":"From storage"}');
            document.getElementById("host").attachShadow({ mode: "open" }).innerHTML = "<h2>In a component</h2>";
          </script>
          <iframe id="ad" src="http://localhost:${server.address().port}/ad"></iframe>
          <iframe id="player" src="http://localhost:${server.address().port}/player"></iframe>
          <script>
            setTimeout(() => { const x = new XMLHttpRequest(); x.open("GET", "/api/xhr?n=1"); x.send(); }, 1500);
          </script>`,
      );
      return;
    }
    if (request.url === "/player" || request.url === "/ad") {
      response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      response.end(`<!doctype html><h1>${request.url === "/player" ? "Inside the frame" : ""}</h1>`);
      return;
    }
    if (request.url.startsWith("/site")) {
      response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      response.end(
        `<!doctype html><title>Site</title><body style="margin:40px;background:#fff"><article class="post">
          <h1 style="font-size:40px">The title of the post</h1>
          <h2>Other heading</h2><h2>Written by someone</h2>
          <a href="/left" style="display:block;font-size:30px"><span>A link with a text</span></a>
          <input id="field" value="secret" style="display:block;font-size:30px">
        </article></body>`,
      );
      return;
    }
    const step = new URL(request.url, "http://x").searchParams.get("step") ?? "";
    const safe = step.replace(/[<>&]/g, "");
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end(
      `<!doctype html><title>Test page</title><body style="margin:0;display:grid;place-items:center;height:100vh;background:#0b0d0e;color:#e8eaed;font:600 32px system-ui;text-align:center;padding:0 40px"><p>${safe}</p><video></video>`,
    );
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  const base = `http://127.0.0.1:${server.address().port}`;

  const bridge = spawn(resolve(ROOT, "target/debug/examples/pages"), ["300"], { stdio: ["ignore", "pipe", "inherit"] });
  let code = "";
  let state = { extensions: 0, pages: [] };
  const seen = [];
  const made = [];
  let buffer = "";
  bridge.stdout.on("data", (chunk) => {
    buffer += chunk;
    const lines = buffer.split("\n");
    buffer = lines.pop();
    for (const line of lines) {
      if (line.startsWith("pair ")) code = line.slice(5).trim();
      else if (line.startsWith("made ")) made.push(JSON.parse(line.slice(5)));
      else {
        state = JSON.parse(line);
        seen.push(state);
      }
    }
  });
  while (!code) await wait(50);
  const [port, token] = code.split(":");

  const failures = [];
  const check = (ok, what) => {
    console.log(`${ok ? "ok  " : "FAIL"} ${what}`);
    if (!ok) failures.push(what);
  };
  const reported = (presence) => state.pages.filter((page) => page.presence === presence).length;
  const until = async (condition, ms = 6000) => {
    const end = Date.now() + ms;
    while (Date.now() < end && !condition()) await wait(50);
    return condition();
  };

  const profile = join(home, "profile");
  const child = spawn(
    browser(),
    [
      ...(show ? ["--window-size=1000,700"] : ["--headless=new"]),
      ...(storeFolder ? ["--lang=en"] : []),
      `--user-data-dir=${profile}`,
      "--remote-debugging-pipe",
      "--enable-unsafe-extension-debugging",
      "--remote-debugging-port=0",
      "--no-first-run",
      "--no-default-browser-check",
      ...(process.platform === "linux" ? ["--no-sandbox"] : []),
      "about:blank",
    ],
    { stdio: ["ignore", "ignore", "ignore", "pipe", "pipe"] },
  );

  try {
    // Chrome no longer takes extensions on its command line: only this pipe can
    // load one. Everything else goes through the debugging port.
    const answered = new Promise((done, fail) => {
      let text = "";
      child.stdio[4].on("data", (chunk) => {
        text += chunk;
        for (const line of text.split(" ").slice(0, -1)) {
          const message = JSON.parse(line);
          if (message.id === 1) message.error ? fail(new Error(message.error.message)) : done(message.result);
        }
      });
      setTimeout(() => fail(new Error("the browser did not load the extension")), 15_000);
    });
    child.stdio[3].write(`${JSON.stringify({ id: 1, method: "Extensions.loadUnpacked", params: { path: copy } })} `);
    console.log(`loaded ${(await answered).id}`);

    const portFile = join(profile, "DevToolsActivePort");
    while (!existsSync(portFile) || !readFileSync(portFile, "utf8").includes("\n")) await wait(100);
    const debug = readFileSync(portFile, "utf8").split("\n")[0].trim();
    const targets = async () => (await fetch(`http://127.0.0.1:${debug}/json/list`)).json();
    const worker = async () => {
      // A worker that just started again can still be listed under its old address.
      for (let i = 0; i < 100; i += 1) {
        // The browser and security programs run workers of their own.
        const found = (await targets()).find(
          (target) => target.url.startsWith(`chrome-extension://${EXTENSION_ID}/`) && target.type === "service_worker",
        );
        if (found) {
          const link = await connect(found.webSocketDebuggerUrl).catch(() => null);
          // A worker that is starting has no extension calls yet.
          for (let j = 0; link && j < 50; j += 1) {
            const { result } = await link.call("Runtime.evaluate", {
              expression: "!!globalThis.chrome?.storage",
              returnByValue: true,
            });
            if (result?.value) return link;
            await wait(100);
          }
          link?.close();
        }
        await wait(100);
      }
      throw new Error("the extension did not start");
    };
    // Writes what the extension keeps. The bridge sends its rules again every two
    // seconds, and that is when the extension reads this.
    const store = async (values, then = "") => {
      const link = await worker();
      const answer = await link.call("Runtime.evaluate", {
        expression: `chrome.storage.local.set(${JSON.stringify(values)}).then(() => { ${then} })`,
        awaitPromise: true,
      });
      if (answer.failed || answer.exceptionDetails) console.log("store:", JSON.stringify(answer).slice(0, 300));
      link.close();
      await wait(2600);
    };
    // A tab of our own: browsers differ in what they open at the start.
    const tab = await (await fetch(`http://127.0.0.1:${debug}/json/new?about:blank`, { method: "PUT" })).json();
    const page = await connect(tab.webSocketDebuggerUrl);
    await page.call("Page.bringToFront");
    const open = async (path, step) => {
      const mark = path.includes("?") ? "&" : "?";
      await page.call("Page.navigate", { url: `${base}${path}${mark}step=${encodeURIComponent(step)}` });
      await wait(show ? 3000 : 500);
    };

    // An alarm is what makes a sleeping extension look for the app.
    await store(
      { pairing: { port: Number(port), token }, openSites: [] },
      `chrome.alarms.create("try", { when: Date.now() + 100 })`,
    );
    if (!(await until(() => state.extensions === 1))) {
      const link = await worker();
      const seen = await link.call("Runtime.evaluate", {
        expression: `(async () => {
          const kept = await chrome.storage.local.get(null);
          const opened = await new Promise((done) => {
            const socket = new WebSocket("ws://127.0.0.1:" + kept.pairing?.port + "/");
            socket.onopen = () => done("socket open");
            socket.onerror = () => done("socket error");
            setTimeout(() => done("socket silent"), 2000);
          });
          return JSON.stringify({ keys: Object.keys(kept), port: kept.pairing?.port, alarms: await chrome.alarms.getAll(), opened });
        })()`,
        awaitPromise: true,
        returnByValue: true,
      });
      console.log(JSON.stringify(seen).slice(0, 500));
      link.close();
      throw new Error("the extension did not reach the bridge");
    }
    check(true, "the extension reaches the bridge");

    // The browser allows this site, but it was not chosen for Web Media.
    await open("/article", "Paso 1: sitio con permiso, no elegido para Web Media. No debe leerlo.");
    await wait(4000);
    check(reported("Web Media") === 0, "Web Media does not read a site chosen for nobody");

    await open("/watch?v=1", "Paso 2: la presencia Video lee su sitio. Web Media no recibe nada.");
    check(await until(() => reported("Video") === 1), "a presence for that site reads it");
    check(reported("Web Media") === 0, "the permission of that presence gives Web Media nothing");

    await store({ openSites: [SITE] });
    await open("/article", "Paso 3: sitio elegido para Web Media. Ahora sí debe leerlo.");
    check(await until(() => reported("Web Media") === 1), "Web Media reads a site chosen for it");

    await open("/watch?v=2", "Paso 4: un sitio con presencia propia va primero.");
    check(
      await until(() => reported("Video") === 1 && reported("Web Media") === 0),
      "a site with its own presence goes first",
    );

    await store({ openSites: [] });
    await open("/article", "Paso 5: se quitó el sitio. Web Media deja de leerlo.");
    await wait(3000);
    check(reported("Web Media") === 0, "Web Media stops when the site is taken out");

    // A page with a huge address must not take the link down.
    await store({ openSites: [SITE] });
    await open("/article", "Paso 6: una dirección gigante no debe cortar la conexión.");
    await until(() => reported("Web Media") === 1);
    const before = seen.length;
    await page.call("Runtime.evaluate", { expression: `history.pushState({}, "", "/" + "x".repeat(400000))` });
    await wait(7000);
    const drops = seen.slice(before).filter((entry) => entry.extensions === 0).length;
    console.log(`     connections lost after a huge address: ${drops}`);
    check(drops === 0, "a huge address leaves the link up");

    // A frame from another site, a list and a page variable reach the app as one page.
    await page.call("Page.navigate", { url: `${base}/framed` });
    const framedPage = () => state.pages.find((item) => item.presence === "Framed")?.values ?? {};
    check(
      await until(() => framedPage().player_title === "Inside the frame"),
      `a frame from another site reaches the app: ${JSON.stringify(framedPage())}`,
    );
    check(JSON.stringify(framedPage().tags) === '["Action","Drama"]', "a list reads every match");
    check(framedPage().episode === 12, "a page variable reaches the app");
    check(JSON.stringify(framedPage().cast) === '["A","B"]', "a list in a page variable reaches the app");
    check(framedPage().call === null, "a function of the page does not, and nothing calls it");
    check(framedPage().next_data === "From JSON", "JSON written in the page reaches the app");
    check(framedPage().poster === "rgb(1, 2, 3)", "a computed style reaches the app");
    check(framedPage().inside === "In a component", "a selector crosses into a component");
    if (!framedPage().stored) await until(() => framedPage().stored === "From storage");
    check(framedPage().stored === "From storage", "the storage of the site reaches the app");
    check(
      framedPage().answer === "From an answer",
      "an answer that came before the extension was ready reaches the app",
    );
    if (framedPage().xhr !== "From XHR") await until(() => framedPage().xhr === "From XHR");
    check(framedPage().xhr === "From XHR", "a later XMLHttpRequest answer reaches the app");
    check(framedPage().page_saw === "From an answer", "the page still receives its own answer");
    check(framedPage().own === "From an answer", "an address that starts with / is read on the site of the page");
    check(!JSON.stringify(framedPage()).includes("stays"), "nothing outside the declared path leaves the page");
    const inner = await page.call("Runtime.evaluate", {
      expression: `document.getElementById("player").contentWindow.location.pathname`,
      returnByValue: true,
    });
    check(
      inner.exceptionDetails || inner.result?.subtype === "error" || inner.result?.value === undefined,
      "the page cannot read into the frame, and the frame stays another site",
    );

    // Clicking texts makes a presence. The popup starts it; here the worker does.
    await page.call("Page.navigate", { url: `${base}/site` });
    await wait(700);
    const center = async (selector) => {
      const { result } = await page.call("Runtime.evaluate", {
        expression: `JSON.stringify((() => { const area = document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect(); return { x: area.left + area.width / 2, y: area.top + area.height / 2 }; })())`,
        returnByValue: true,
      });
      return JSON.parse(result.value);
    };
    const click = async (selector) => {
      const { x, y } = await center(selector);
      await page.call("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
      await page.call("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", clickCount: 1 });
      await page.call("Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button: "left", clickCount: 1 });
      await wait(150);
    };
    const link = await worker();
    const started = await link.call("Runtime.evaluate", {
      expression: `(async () => {
        const [tab] = await chrome.tabs.query({ url: "http://127.0.0.1/*" });
        return JSON.stringify(await startPick(tab.id));
      })()`,
      awaitPromise: true,
      returnByValue: true,
    });
    check(started.result?.value === '{"ok":true}', `the pick starts on the tab: ${started.result?.value}`);
    await wait(300);
    await click("#field");
    await click("h1");
    await click("h2:nth-of-type(2)");
    const before2 = await page.call("Runtime.evaluate", { expression: "location.pathname", returnByValue: true });
    await page.call("Input.dispatchKeyEvent", {
      type: "keyDown",
      key: "Enter",
      code: "Enter",
      windowsVirtualKeyCode: 13,
      text: "\r",
    });
    await page.call("Input.dispatchKeyEvent", {
      type: "keyUp",
      key: "Enter",
      code: "Enter",
      windowsVirtualKeyCode: 13,
    });
    check(await until(() => made.length === 1), "the clicked texts reach the app");
    const request = made[0] ?? {};
    console.log(`     ${JSON.stringify(request)}`);
    check(request.host === "127.0.0.1", "the site comes from the tab");
    check(request.first === "h1", "a form field is refused and the title is taken");
    const second = await page.call("Runtime.evaluate", {
      expression: `document.querySelector(${JSON.stringify(request.second ?? "nothing")})?.textContent`,
      returnByValue: true,
    });
    check(second.result?.value === "Written by someone", "the second line finds exactly what was clicked");
    check(before2.result?.value === "/site", "the page stays where it was");

    // A link is a text like any other, and clicking it must not follow it.
    await page.call("Page.navigate", { url: `${base}/site` });
    await wait(700);
    await link.call("Runtime.evaluate", {
      expression: `(async () => { const [tab] = await chrome.tabs.query({ url: "http://127.0.0.1/*" }); await startPick(tab.id); })()`,
      awaitPromise: true,
    });
    await wait(300);
    await click("a span");
    await wait(300);
    const stayed = await page.call("Runtime.evaluate", { expression: "location.pathname", returnByValue: true });
    check(stayed.result?.value === "/site", "clicking a link while choosing does not follow it");

    // The real popup, opened by the browser on this page, starts it with its button.
    await page.call("Page.navigate", { url: `${base}/site` });
    await wait(700);
    await page.call("Page.bringToFront");
    const shown = await link.call("Runtime.evaluate", {
      expression: `chrome.action.openPopup().then(() => "open", (error) => error.message)`,
      awaitPromise: true,
      returnByValue: true,
    });
    check(shown.result?.value === "open", `the browser opens the popup: ${shown.result?.value}`);
    let popupTarget;
    for (let i = 0; i < 40 && !popupTarget; i += 1) {
      popupTarget = (await targets()).find((target) => target.url.endsWith("/popup.html"));
      await wait(100);
    }
    if (popupTarget) {
      const popup = await connect(popupTarget.webSocketDebuggerUrl);
      const inPopup = async (expression) =>
        (await popup.call("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true })).result?.value;
      let offered = false;
      for (let i = 0; i < 40 && !offered; i += 1) {
        offered = await inPopup(`!document.getElementById("makeRow").hidden`);
        await wait(100);
      }
      check(offered, "the popup offers to make a presence on a page only Web Media reads");
      await inPopup(`document.getElementById("makeSite").click()`);
      await wait(800);
      popup.close();
      const added = await page.call("Runtime.evaluate", {
        expression: `[...document.documentElement.children].some((item) => item.tagName === "DIV" && item.shadowRoot === null)`,
        returnByValue: true,
      });
      check(added.result?.value === true, "its button starts the picker, in a shadow root the page cannot open");
      await page.call("Input.dispatchKeyEvent", {
        type: "keyDown",
        key: "Escape",
        code: "Escape",
        windowsVirtualKeyCode: 27,
      });
      await page.call("Input.dispatchKeyEvent", {
        type: "keyUp",
        key: "Escape",
        code: "Escape",
        windowsVirtualKeyCode: 27,
      });
      await wait(300);
      const left = await page.call("Runtime.evaluate", {
        expression: `[...document.documentElement.children].some((item) => item.tagName === "DIV")`,
        returnByValue: true,
      });
      check(left.result?.value === false, "Escape leaves the page as it was");
    } else {
      check(false, "the popup page opens");
    }

    if (storeFolder) {
      // 640 x 400 at twice the density is the 1280 x 800 the stores ask for.
      const { result: id } = await link.call("Runtime.evaluate", {
        expression: "chrome.runtime.id",
        returnByValue: true,
      });
      await page.call("Page.navigate", { url: `chrome-extension://${id.value}/popup.html` });
      await page.call("Emulation.setDeviceMetricsOverride", {
        width: 640,
        height: 400,
        deviceScaleFactor: 2,
        mobile: false,
      });
      await wait(1200);
      await page.call("Runtime.evaluate", {
        expression: `document.body.style.margin = "0 auto"; document.documentElement.style.background = getComputedStyle(document.body).backgroundColor;`,
      });
      const { data } = await page.call("Page.captureScreenshot", { format: "png" });
      mkdirSync(storeFolder, { recursive: true });
      writeFileSync(join(storeFolder, "popup.png"), Buffer.from(data, "base64"));
      console.log(`     saved ${join(storeFolder, "popup.png")}`);
    }
    page.close();
  } finally {
    child.kill();
    bridge.kill();
    server.close();
    await wait(800);
    try {
      rmSync(home, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 });
    } catch {
      console.warn(`left behind: ${home}`);
    }
  }
  console.log(failures.length ? `${failures.length} failed` : "all passed");
  process.exit(failures.length ? 1 : 0);
}

await main();
