---
title: Functions
description: How a Rich Presence Plus script works, what read, activity and meta receive and return, and the helpers every presence can call.
---

# Functions

A script defines functions, and the application calls them. You never call them yourself, and
you never talk to Discord.

## How a presence runs

Every `interval` seconds the application does this:

1. It calls `read`. Your code looks at the world and returns what it found.
2. If `read` returned `null`, nothing is showing and the round ends here.
3. It calls `activity` with what `read` returned. Your code decides what Discord shows.
4. It calls `meta` with the same value, if you wrote one.
5. It applies the person's privacy rules, compares with the other presences and sends the
   winner to Discord.

Two functions keep things simple: `read` gathers, `activity` presents. When a layout changes
on a site, you touch `read`. When you want a new button, you touch `activity`.

## read(context)

`read` receives what the application knows and returns a plain object with what you need, or
`null` when there is nothing to show.

```js
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;
  return { title: page.title, opened: context.now };
}
```

| Field of `context` | What it holds |
| --- | --- |
| `now` | The time, in Unix seconds. |
| `pages` | Pages the extension reported for this presence. |
| `options` | The value of each option. |

Each page has `url`, `title`, `received_at` and `values`, the data you asked for in the
manifest.

Variables outside the functions stay between calls. That is how a script remembers things,
such as when something started:

```js
let since = 0;

function read(context) {
  const running = host.process.list().includes("game.exe");
  if (!running) {
    since = 0;
    return null;
  }
  if (since === 0) since = context.now;
  return { since };
}
```

## activity(seen)

`activity` receives what `read` returned and describes the card. Every field is optional, but
the card needs at least a text or an image.

```js
function activity(seen) {
  return {
    details: cut(seen.title),
    state: t("browsing"),
    kind: "watching",
    start: seen.opened,
    buttons: [{ label: t("open"), url: seen.url }],
  };
}
```

| Field | What it does |
| --- | --- |
| `name` | The title of the activity, in place of the Discord application's name. |
| `details`, `state` | The two lines of text, up to 128 characters. Shorter than two characters is dropped. |
| `kind` | `playing`, `listening`, `watching` or `competing`. |
| `start`, `end` | Unix seconds. `start` alone counts up; with `end` it draws a progress bar. |
| `large_image`, `small_image` | An `https://` address, or the name of an image uploaded to your Discord application. |
| `large_text`, `small_text` | Text shown when hovering the images. |
| `buttons` | Up to two `{ label, url }`. |
| `details_url`, `state_url` | Address opened when clicking that line. |
| `large_url`, `small_url` | Address opened when clicking that image. |
| `party_size` | `[current, largest]`, shown as "(2 of 4)". |
| `status_display` | Which text the member list shows: `name`, `state` or `details`. |

## meta(seen)

`meta` is optional. It tells the application things Discord does not show, which help it
choose between presences and apply the privacy rules.

```js
function meta(seen) {
  return { paused: seen.paused, application: "game.exe" };
}
```

| Field | What it does |
| --- | --- |
| `paused` | A paused presence gives way to one that is playing, and hides by default. |
| `foreground` | Ranks the presence higher while its window has focus. |
| `application` | Program name, so people can exclude it in Privacy. |
| `domains` | Extra sites, so people can exclude them in Privacy. |

To keep a presence on display while paused, add `"hide_when_paused": false` to the manifest.

## Helpers

Every script can call these with nothing to import.

| Helper | What it does |
| --- | --- |
| `t(key, values)` | Gives the text for `key` in the person's language. |
| `cut(text, max)` | Shortens a text to `max` characters (128 by default). |
| `parseUrl(address)` | Gives `{ host, path, query }`, or `null`. There is no `URL` class. |
| `bestPage(pages)` | Picks the page with a running player, or else the first one. |
| `playback(player, receivedAt)` | Gives `{ playing, start, end }` for a `media` value. |
| `parseTime(text)` | Reads `1:02:03`, `4:05` or `30` as seconds, or gives `null`. |
| `progress(current, duration, now)` | Gives `{ start, end }` for a position and a length in seconds. |
| `console.log(...)` | Writes a line in the Console block of the Developer page. |

For what `host` holds, see [Desktop Presences](06-desktop.md).

## Limits

A script has 16 MiB of memory and 2 seconds per call, and `read` must return less than
64 KiB. Variables persist, so a cache is fine, but limit how much it keeps.
