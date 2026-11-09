# Developing

What you need to know to work on the application, past what the code says. For how to
contribute see [CONTRIBUTING.md](../CONTRIBUTING.md), and for how to write a presence see
[developers](developers/en/01-home.md).

## Layout

| Path | What it holds |
| --- | --- |
| `crates/rpp-core` | Activity model, validation, priorities, privacy, backoff. Pure code, no I/O |
| `crates/rpp-discord` | Discord IPC framing, client and reconnecting connection |
| `crates/rpp-plugins` | Manifest, permissions and the sandboxed JavaScript engine (`read` then `activity`) |
| `crates/rpp-host` | Native sources for scripts: processes, foreground window, media session, log files, HTTP |
| `crates/rpp-service` | One thread per script, the policy that picks an activity, one Discord connection |
| `crates/rpp-bridge` | Local WebSocket server for the browser extension: loopback only, paired by token |
| `crates/rpp-config` | Settings saved atomically, presence folders loaded from disk |
| `crates/rpp-catalog` | Signed catalog: verify the index, install from GitHub, keep two earlier versions |
| `crates/rpp-app` | Controller behind the window: settings, presences, custom status, profiles, developer tools |
| `crates/rpp-testkit` | Test doubles shared by the workspace |
| `src-tauri/` | Tauri shell: window, tray icon, single instance, commands |
| `frontend/` | Preact interface |
| `extension/` | Browser extension (Manifest V3) that reads the values a presence declares |
| `presences/` | Presences that ship with the application and the catalog |
| `website/` | Public site in Astro, built from the manifests and the latest release |

## Building

| What | Command |
| --- | --- |
| Interface | `pnpm --dir frontend build` |
| Program for testing | `cargo build -p rpp-desktop --features custom-protocol` |
| Program, final | `cargo build --release -p rpp-desktop --features custom-protocol` |
| Installer, unsigned | `cargo tauri build` |
| Website | `pnpm --dir website build` |
| Extension zip | `node tools/package-extension.mjs --out dist` |

Build the interface before the program, because the program embeds it. Without
`custom-protocol` the window looks for a development server and shows an error page. A
running program locks its executable, so close it before a release build.

When disk space is short, `CARGO_INCREMENTAL=0` keeps the build from growing
`target/debug/incremental`.

## Checks

The list in [CONTRIBUTING.md](../CONTRIBUTING.md) must pass before something counts as done.
These go further, and they run real browsers:

| Tool | What it proves |
| --- | --- |
| `tools/try-interface.mjs` | Every page, form and keyboard path, under the window's real CSP, against a made-up desktop side |
| `tools/try-extension.mjs` | The extension in Chrome: rules, frames, lists, variables, the picker and the popup. `--show` to watch, `--store docs/store` for the store picture |
| `tools/try-browsers.mjs` | Automatic pairing in real browsers |
| `tools/try-memory.mjs` | Memory of the release build through idle, moves and the tray |
| `tools/thread-cpu.ps1` | What each thread costs, by name |
| `tools/soak.ps1` | Hours of memory readings |

`crates/rpp-service/examples/discord_check.rs` holds an activity on the real Discord, and
`two_activities.rs` holds two at once. `crates/rpp-bridge/examples/pages.rs` is the bridge that
`try-extension` talks to: a new extractor to try goes in its `framed` rule.

## Reaching the real application

The real window can be driven with no clicks. Start the program with a debugging port:

```sh
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--disable-gpu --remote-debugging-port=9444" ./target/release/rpp-desktop.exe
```

Then `http://127.0.0.1:9444/json/list` gives the page, and any command can be called through
`Runtime.evaluate` with `window.__TAURI_INTERNALS__.invoke("presences", {})`. Use `--hidden`
to start in the tray, with no window.

## Values worth knowing

| What | Value |
| --- | --- |
| Data folder | `%LOCALAPPDATA%\com.manucabral.richpresenceplus` |
| In it | `settings.json`, `usage.json`, `presences/`, `workbench/<folder>/versions`, logs |
| Shared Discord application | `1567580382377803786` |
| Local address | `127.0.0.1:28752`, off by default |
| Ticker (`src-tauri/src/idle.rs`) | Every 5 s: idle, rotation, quiet rule, pages' requests, folder look, time on display |
| Sending an activity again | Every 15 s, so a restarted Discord gets it |
| Processes listed | At most every 2 s for the window, 15 s for the quiet rule, 30 s for the tray |
| Window hidden | Destroyed after 10 s, memory handed back after that |
| Folder watcher | Looks every 250 ms, reports after 500 ms of quiet |
| Time on display | Written once a minute, kept 56 days; a gap over 30 s does not count |

## Adding something

**A new extractor** touches `crates/rpp-plugins/src/page.rs` (with its limits),
`extension/extract.js`, `crates/rpp-app/templates/manifest.schema.json`, `value_type` in
`crates/rpp-app/src/editor.rs`, both `09-extractors.md`, and a check in `tools/try-extension.mjs`.

**A new activity field** touches `crates/rpp-core/src/activity.rs`, `activity_json` in
`crates/rpp-discord/src/protocol.rs`, `filter_activity` if it can carry a title, the interface
types, `script-api.d.ts` and both `10-script.md`.

**A new permission** touches `Permissions` in `crates/rpp-plugins/src/permissions.rs`, where
`grants()` fails to compile until it lists it, so an update that adds it asks first. Then
`prelude` in `engine.rs`, `Access` in `views.rs`, the `host` part of `editor.rs`, the schema and
both `11-host.md`.

**A new tag** goes in `crates/rpp-plugins/src/tags.rs`, `manifest.schema.json`,
`frontend/src/shared/tags.ts`, both locale files and both `08-manifest.md`. Tests compare the
first three, and `website/test/docs.test.ts` checks the pages. Removing one breaks presences that use it in the catalog check: add, do not rename.

**A new page of the interface** goes in `ROUTES` in `frontend/src/main.tsx` and `navItems` in
`Layout.tsx`, and in `routes` in `tools/try-interface.mjs` so it gets checked.

**A new text** goes in both `frontend/src/locales` files, and `tools/check-locales.mjs` checks
them. The documentation has the same parts and code blocks in both languages, which
`website/test/docs.test.ts` checks, along with these: the reference names every field of
`manifest.schema.json` and `script-api.d.ts`, headings start with the only capital, and every
link lands on a heading that exists. The tutorial's last block must equal `WEB` in
`crates/rpp-host/tests/docs_examples.rs`; its page is `website/src/components/Practice.astro`,
whose class names the tutorial uses.

## Decisions

- Presences are JavaScript files with a manifest. They run in an embedded engine with no file,
  network or process access except what the manifest declares and the person accepts.
- Web services are read by the extension, which reaches the app over a local WebSocket
  protected by a token. No debugging port is opened.
- Desktop presences use sources the Rust host exposes, so a script never needs native code.
- One component owns the Discord connection. Presences only offer candidates; the policy in
  `rpp-core` picks one and applies the privacy rules.
- Starting with Windows writes `RichPresencePlus` under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, for this user only, and starts the
  program with `--hidden`. The installer asks once; if it did not, the first run asks.
  Uninstalling removes the value.
- A presence declares what it reads. The extension runs fixed code and never code of a
  presence, so a page can only be read, never changed.
- Permissions are given site by site. A presence for every site reads only the sites chosen for
  it, and a permission given to one presence hands nothing to another.
- `name` on an activity lets presences share one Discord application. Discord accepted it over
  the local pipe on 2026-11-05 and answers with it. Hiding titles hides it too.
- Time on display is off until the person turns it on, and the local address too.
- A name that looks like a login is refused when a manifest loads, as a floor under the review.
- The website runs no script unless a page names one, and loads nothing from other servers.
- Markdown links in `docs/developers` point at files, so they work on GitHub, and
  `website/src/components/DocLinks.astro` turns them into addresses of the site.

## Traps

- In a headless browser, one Tab past the last control leaves the page for the browser's bar, and
  the keys after it are lost. A test must stop at the last control. The real window has no bar.
- A key sent over the debugging port and a question sent after it travel apart. Record focus with
  `focusin` inside the page, and never by asking after each key.
- Rows that look rebuilt usually are not: prove it before changing the code.
- Astro 7 no longer brings the Markdown plugins, and adding them means a dependency.
- A closed shadow root cannot be read, by design of the browser.
