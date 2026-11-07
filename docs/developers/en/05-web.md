---
title: Web Presences
description: Build a presence for a website step by step, reading the page through the browser extension and showing the title, a button and a progress bar.
---

# Web Presences

A web presence shows what you do on a site. Your script does not run inside the page. The
extension reads the values you name in the manifest and hands them to your script, so a page
can never run your code, and your code can never touch the page.

This page builds a presence for a video site, one piece at a time.

## 1. Say which pages to read

`matches` lists the addresses the presence works on. `*` matches anything.

```json
"page": {
  "matches": ["*://www.example.com/watch*"],
  "extract": {}
}
```

## 2. Say what to read

Each entry of `extract` takes one value from the page, under a name you choose.

```json
"extract": {
  "title": { "from": "text", "selector": "h1.video-title" },
  "channel": { "from": "text", "selector": "a.channel-name" },
  "channel_url": { "from": "attr", "selector": "a.channel-name", "name": "href" },
  "player": { "from": "media", "selector": "video" }
}
```

To find a selector, open the site, right-click the element and choose Inspect. In the tools,
right-click its line, then Copy, Copy selector.

| `from` | What it gives you |
| --- | --- |
| `text` | The visible text of the first element that matches `selector`. |
| `attr` | The attribute `name` of that element, such as a link's `href`. |
| `meta` | The content of the `<meta>` tag with that `name` or `property`. |
| `media` | `{ paused, current, duration, rate }` of a `<video>` or `<audio>`. |
| `session` | `{ title, artist, album, artwork, state }` from the site's Media Session. |
| `list` | Every element that matches, as a list. Add `name` to read an attribute, and `max` for a limit (16 by default, at most 32). |
| `variable` | What the page keeps in a global variable, by its `path`. |
| `json` | JSON written in an element, optionally by its `path`. |
| `storage` | An entry of `localStorage`, or of `sessionStorage` with `"area": "session"`, by its `key`. |
| `style` | A computed CSS property of an element, by its `name`. |

## 3. Read the values

The values arrive in `page.values`, under the names you chose. Sites change, so any value can
be empty: use a fallback.

```js
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;

  const values = page.values;
  return {
    title: values.title || page.title,
    channel: values.channel,
    channel_url: values.channel_url,
    url: page.url,
    ...playback(values.player, page.received_at),
  };
}
```

`bestPage` chooses when several tabs match, and it prefers the one that is playing. It looks
for the value named `player`, so use that name for your `media` extractor. `playback` turns
that value into `playing`, `start` and `end`, and the `...` adds those three to the object.

## 4. Show it

```js
function activity(video) {
  const buttons = [{ label: t("watch"), url: video.url }];
  if (video.channel_url) buttons.push({ label: t("channel"), url: video.channel_url });

  return {
    details: cut(video.title),
    state: video.channel || t("watching"),
    kind: "watching",
    start: video.start,
    end: video.end,
    buttons,
  };
}
```

`cut` keeps long titles under Discord's limit. `start` and `end` draw the progress bar, and
they are empty while paused, so the bar goes away by itself. The channel button is added only
if the link exists.

## 5. Report when it is paused

```js
function meta(video) {
  return { paused: !video.playing };
}
```

With this, a song playing somewhere else wins over your paused video.

## 6. Add the texts and try it

```json
{
  "description": "Shows the video you are watching.",
  "watch": "Watch",
  "channel": "Channel",
  "watching": "Watching"
}
```

Save, press Watch in the Developer page and open a video. If the preview stays empty, the site
needs permission: open the extension and press "Allow this site".

## When the video is in a frame

Many sites put their player in an iframe from another site. A selector cannot reach inside it,
so `page.frames` says which frames to read and what to take from them:

```json
"page": {
  "matches": ["*://www.example.com/watch*"],
  "extract": {
    "title": { "from": "text", "selector": "h1" }
  },
  "frames": {
    "matches": ["*://player.example.net/*"],
    "extract": {
      "player": { "from": "media" }
    }
  }
}
```

Your script does not change: `player` arrives in `page.values` next to `title`, and `bestPage`
and `playback` work the same.

To find the address of the frame, right-click the player, choose Inspect and look for the
`<iframe>` around it. Each pattern must name a site, so `*://*/*` is not allowed. People give
permission for the page and its frames with one click. A frame only reads inside the pages of
its own presence, and frames that match nothing cost nothing.

## Lists

`list` reads every match, which helps when a page has several of something:

```json
"genres": { "from": "list", "selector": "a.genre", "max": 3 }
```

```js
const genres = page.values.genres;         // ["Action", "Drama", "Comedy"]
const state = genres.join(", ");           // "Action, Drama, Comedy"
```

## Data the page does not show

The text on screen is often not the best source. Sites keep cleaner data in other places, and a
presence can read all of them without running anything inside the page.

**Page variables.** To see what a site has, open the browser's Console and type `window.` to
explore it. Then ask for the path. It gives a text, a number, a boolean, or a small list or
object.

```json
"episode": { "from": "variable", "path": "app.video.episode" }
```

**JSON in the page.** Many sites write their data in a script tag. Search the page's source for
`__NEXT_DATA__` or `application/ld+json`.

```json
"title": { "from": "json", "selector": "script#__NEXT_DATA__", "path": "props.pageProps.title" }
```

**Storage.** Music players often save the current song. Look in the Application tab of the
browser's tools.

```json
"song": { "from": "storage", "key": "player-state", "path": "track.title" }
```

**Styles.** Some sites set a picture as a background instead of an image.

```json
"cover": { "from": "style", "selector": ".poster", "name": "background-image" }
```

**Components.** A selector cannot see inside a web component. Write `>>>` to cross into it:

```json
"title": { "from": "text", "selector": "video-player >>> h2" }
```

## What a presence cannot read

A path is names with dots, so it cannot call anything. A name that holds `token`, `password`,
`secret`, `cookie` and the like is refused when the presence loads, so a presence cannot read a
login. Read only what you show: reviewers check each path.

## The real thing

`presences/youtube` is this same presence for a real site, and `presences/youtube-music` reads
the `session` value instead of selectors. Both are in the repository.

## Without code

To start faster, open the extension on a site and press "Make a presence". Click the texts you
want and give it a name. The application writes the `matches` and the `extract` for you, and
you can keep editing from there.
