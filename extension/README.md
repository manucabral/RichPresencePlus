# Browser extension

Reads the values a presence declares in its `manifest.json` (`page.extract`)
and sends them to the desktop application over a local WebSocket. It never
runs code received from the application: it only evaluates selectors, meta
tags and media state with fixed code in `extract.js`.

## Load it for development

1. Open `chrome://extensions` (or `edge://extensions`) and turn on developer mode.
2. Choose "Load unpacked" and select this folder.
3. Start the desktop app once. It registers a native messaging host, and the
   extension asks it for the port and the token by itself.
4. When a presence needs access to a site, the popup lists it; choose "Allow".

If the popup says it cannot find the app, paste the pairing code from the
app's Browser page (`port:token`) under "Pair with a code".

## How it finds the app

The `key` in `manifest.json` fixes the extension ID of an unpacked copy
(`iijcphjgahgdkdeaibcgagjfoaedajjk`). The Chrome Web Store copy has
`cjdkaikgiokfekdbckccmlbffinaknib`, and Firefox uses the add-on ID
`richpresenceplus@manucabral.dev` from the manifest. The desktop app writes host
manifests that allow only those IDs, and points Chrome, Edge and
Firefox to them in `HKEY_CURRENT_USER`. The browser starts the app with the
extension's origin; the app answers with the port and token it recorded in
`pairing.json` and exits without opening a window. The protocol is in
`crates/rpp-bridge/src/native.rs`.

The stores assign their own IDs and reject a manifest with a `key`, so the
packaged extension drops it. A store ID pairs by itself only with an app
version that lists it; older versions connect with the pairing code.

## The popup

- The link to the app, with the browser and version it detected, how long it
  has been connected and how many tabs it reads.
- **Pause this browser**: pages are still read but nothing is sent, and what
  was sent is taken back. Resuming sends the open pages again.
- **This page**: which presence reads the page the popup was opened on, and a
  button to allow that site when it needs access. `activeTab` gives the address
  of that one tab while the popup is open; no warning at install.
- The presences the app asked this browser to read, with whether each has
  access to its sites.
- Pairing by hand, as a fallback.

Texts are in `_locales/en` and `_locales/es`; the browser picks the language.

## Files

| File | Role |
| --- | --- |
| `background.js` | Service worker: connection, content script registration, forwarding. |
| `content.js` | Runs on matching pages and reports changes. |
| `extract.js` | Pure functions: match patterns, extractors, change detection. |
| `pairing.js` | Pairing code, native host answers, reconnection wait, host permission patterns. |
| `popup.html`, `popup.js` | The popup described above. |
| `status.js` | Pure helpers for the popup: messages, matching the page, times. |
| `browser.js` | Which browser this is, and its version. |

## Tests

```
pnpm install
pnpm test
```
