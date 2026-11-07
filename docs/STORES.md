# Publishing the extension

Steps and texts for the Chrome Web Store, Edge Add-ons and
addons.mozilla.org.

## Package

```
node tools/package-extension.mjs --out dist
```

It writes `dist/rich-presence-plus-extension-<version>.zip` with the manifest,
the popup, the top-level scripts, `_locales` and `icons`, and nothing else. The
manifest loses its `key`: the stores refuse it and assign their own ID. Add
each store's ID to `CHROMIUM_EXTENSIONS` in `crates/rpp-bridge/src/native.rs`
before releasing the app, or the extension cannot find the app by itself.

The icons in `extension/icons` are the app logo scaled to 16, 32, 48 and
128 px; the 128 px one is also the store icon. Each store asks for at least one
screenshot of the popup: `store/popup.png` is 1280x800, made with
`node tools/try-extension.mjs --store docs/store`.

## Single purpose

Shows on Discord what the user watches or listens to on sites they choose,
through the Rich Presence Plus desktop app.

## Permissions and why

| Permission | Why |
| --- | --- |
| `storage` | Keeps the pairing with the desktop app, the pause switch and the rules the app sent. |
| `scripting` | Registers the page scripts for the sites of the presences the user turned on, including a fixed script that reads the values the presence names from the page's own variables. |
| `alarms` | Tries to reach the desktop app again every 30 seconds while it is closed. |
| `nativeMessaging` | Asks the desktop app, which registers itself as a native host, for its local port and key, so the user does not copy a code. |
| `activeTab` | Shows in the popup whether a presence reads the page it was opened on. |
| `optional_host_permissions: *://*/*` | Never granted at install. The popup asks for one site at a time, when a presence the user turned on needs it. |

No remote code: every script is in the package, and the page scripts only
evaluate selectors and media state with fixed code (`extract.js`).

## Data

What leaves the browser, and where it goes:

- To the desktop app on the same computer, over `127.0.0.1`, authenticated
  both ways and encrypted: the address, title and the values a presence
  declares (such as a video title or the playback position) of tabs on sites
  the user allowed, for presences the user turned on.
- Nothing from private windows, and nothing while the user paused the browser.
- The extension sends nothing to any server. The desktop app shows an activity
  on the user's Discord profile through Discord's local connection, after its
  own privacy rules (excluded sites, hidden titles, pause).

Firefox: the manifest declares `browsingActivity` and `websiteContent` in
`data_collection_permissions`, so Firefox asks the user before installing
(Firefox 140 or later).

Chrome Web Store "privacy practices": website content and web history are
collected, used only for the single purpose above, not sold, not transferred
to third parties, not used for creditworthiness or lending.

Privacy policy: [docs/legal](legal/privacy.en.md). Both stores need it as a
public URL: `https://manucabral.github.io/RichPresencePlus/privacy/`, or the
same page on the custom domain.

## After the stores accept it

Each store gives the extension its own ID (Chrome and Edge differ). Add both
to `CHROMIUM_EXTENSIONS` in `crates/rpp-bridge/src/native.rs` and release the
app, or the published extension cannot find the app by itself and asks for
the pairing code. Firefox keeps the ID in the manifest
(`richpresenceplus@manucabral.dev`), so nothing changes for it.
