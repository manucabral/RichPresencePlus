// Tries the readers of the extension in a real Firefox, against a bridge of its own.
//
//   node tools/try-firefox.mjs [--show]
//
// The same pages and rules as `try-extension.mjs`, without the picker steps. Firefox
// runs part of the extension its own way (scripts in the page's world, the background
// as an event page, frames of other sites), so the readers are checked here too. It
// runs headless unless --show asks for a window.
import { spawn, spawnSync } from "node:child_process";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { CONTROL_SITE, startFirefox } from "./firefox.mjs";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const SITE = "http://127.0.0.1/*";
const wait = (ms) => new Promise((done) => setTimeout(done, ms));

function testSite() {
  const server = createServer((request, response) => {
    const html = (text) => {
      response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      response.end(text);
    };
    const other = `http://localhost:${server.address().port}`;
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
      html(
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
          <iframe id="ad" src="${other}/ad"></iframe>
          <iframe id="player" src="${other}/player"></iframe>
          <script>
            setTimeout(() => { const x = new XMLHttpRequest(); x.open("GET", "/api/xhr?n=1"); x.send(); }, 1500);
          </script>`,
      );
    } else if (request.url === "/player" || request.url === "/ad") {
      html(`<!doctype html><h1>${request.url === "/player" ? "Inside the frame" : ""}</h1>`);
    } else {
      const step = new URL(request.url, "http://x").searchParams.get("step") ?? "";
      html(`<!doctype html><title>Test page</title><p>${step.replace(/[<>&]/g, "")}</p><video></video>`);
    }
  });
  return new Promise((done) => server.listen(0, "127.0.0.1", () => done(server)));
}

async function main() {
  const show = process.argv.includes("--show");
  const built = spawnSync("cargo", ["build", "-q", "-p", "rpp-bridge", "--example", "pages"], {
    cwd: ROOT,
    stdio: "inherit",
  });
  if (built.status !== 0) process.exit(1);

  const home = mkdtempSync(join(tmpdir(), "rpp-firefox-"));
  const copy = join(home, "extension");
  cpSync(join(ROOT, "extension"), copy, { recursive: true, filter: (path) => !path.includes("node_modules") });
  const manifest = JSON.parse(readFileSync(join(copy, "manifest.json"), "utf8"));
  // Wider than the sites chosen, as when a presence for every subdomain asked first.
  manifest.host_permissions = [SITE.replace("http:", "*:"), "http://localhost/*", CONTROL_SITE];
  writeFileSync(join(copy, "manifest.json"), JSON.stringify(manifest));

  const server = await testSite();
  const base = `http://127.0.0.1:${server.address().port}`;
  const bridge = spawn(resolve(ROOT, "target/debug/examples/pages"), ["300"], { stdio: ["ignore", "pipe", "inherit"] });
  let code = "";
  let state = { extensions: 0, pages: [] };
  const seen = [];
  let buffer = "";
  bridge.stdout.on("data", (chunk) => {
    buffer += chunk;
    const lines = buffer.split("\n");
    buffer = lines.pop();
    for (const line of lines) {
      if (line.startsWith("pair ")) code = line.slice(5).trim();
      else if (line.startsWith("{")) {
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
  const until = async (condition, ms = 8000) => {
    const end = Date.now() + ms;
    while (Date.now() < end && !condition()) await wait(50);
    return condition();
  };

  let firefox = null;
  try {
    firefox = await startFirefox({ copy, profile: join(home, "profile"), show });
    const { order, run, navigate } = firefox;
    console.log(`loaded ${firefox.id}`);
    // The bridge sends its rules again every two seconds, and that is when the
    // extension reads this.
    const store = async (values, alarm = false) => {
      await order({ store: values, alarm });
      await wait(2600);
    };
    const tab = await firefox.tab();
    const open = async (path, step) => {
      const mark = path.includes("?") ? "&" : "?";
      await navigate(tab, `${base}${path}${mark}step=${encodeURIComponent(step)}`);
      await wait(show ? 3000 : 500);
    };

    // An alarm is what makes a sleeping extension look for the app.
    await store({ pairing: { port: Number(port), token }, openSites: [] }, true);
    // The extension may be asking the native host, which is missing here; the next
    // alarm comes within a minute.
    if (!(await until(() => state.extensions === 1, 70_000))) {
      console.log(`     status: ${JSON.stringify(await order({ status: true }))}`);
      throw new Error("the extension did not reach the bridge");
    }
    check(true, "the extension reaches the bridge");
    const facts = await order({ status: true });
    console.log(`     the extension sees ${JSON.stringify(facts.browser)}`);

    await open("/article", "Step 1: allowed, not chosen for Web Media");
    await wait(4000);
    check(reported("Web Media") === 0, "Web Media does not read a site chosen for nobody");

    await open("/watch?v=1", "Step 2: the Video presence reads its site");
    check(await until(() => reported("Video") === 1), "a presence for that site reads it");
    check(reported("Web Media") === 0, "the permission of that presence gives Web Media nothing");

    await store({ openSites: [SITE] });
    await open("/article", "Step 3: chosen for Web Media");
    check(await until(() => reported("Web Media") === 1), "Web Media reads a site chosen for it");

    await open("/watch?v=2", "Step 4: a site with its own presence goes first");
    check(
      await until(() => reported("Video") === 1 && reported("Web Media") === 0),
      "a site with its own presence goes first",
    );

    await store({ openSites: [] });
    await open("/article", "Step 5: the site was taken out");
    await wait(3000);
    check(reported("Web Media") === 0, "Web Media stops when the site is taken out");

    await store({ openSites: [SITE] });
    await open("/article", "Step 6: a huge address");
    await until(() => reported("Web Media") === 1);
    const before = seen.length;
    await run(tab, `history.pushState({}, "", "/" + "x".repeat(400000))`);
    await wait(7000);
    const drops = seen.slice(before).filter((entry) => entry.extensions === 0).length;
    console.log(`     connections lost after a huge address: ${drops}`);
    check(drops === 0, "a huge address leaves the link up");

    // A frame from another site, a list and a page variable reach the app as one page.
    await navigate(tab, `${base}/framed`);
    const framedPage = () => state.pages.find((item) => item.presence === "Framed")?.values ?? {};
    check(
      await until(() => framedPage().player_title === "Inside the frame"),
      `a frame from another site reaches the app: ${JSON.stringify(framedPage())}`,
    );
    check(JSON.stringify(framedPage().tags) === '["Action","Drama"]', "a list reads every match");
    // Checked with the first report: a variable that comes later means the page's
    // world started after the content script.
    check(framedPage().episode === 12, "a page variable reaches the app with the first report");
    if (framedPage().episode !== 12) {
      const inPage = await run(
        tab,
        `(() => {
          let answer = null;
          const take = (event) => { answer = event.detail; };
          document.addEventListener("rpp-values", take);
          document.dispatchEvent(new CustomEvent("rpp-read", { detail: JSON.stringify(["app.video.episode"]) }));
          document.removeEventListener("rpp-values", take);
          return JSON.stringify({ loaded: Boolean(window[Symbol.for("rpp.variables")]), answer });
        })()`,
      );
      console.log(`     in the page's world: ${inPage.value ?? inPage.failed}`);
    }
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
    const inner = await run(tab, `document.getElementById("player").contentWindow.location.pathname`);
    check(Boolean(inner.failed), "the page cannot read into the frame, and the frame stays another site");

    // A browser that closes leaves the app with none of its pages.
    await firefox.close();
    firefox = null;
    check(await until(() => state.extensions === 0), "closing the browser takes its pages back");
  } finally {
    await firefox?.close();
    bridge.kill();
    server.close();
    await wait(1500);
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
