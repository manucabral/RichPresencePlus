---
title: Read a site
description: Find the selectors, frames, lists and hidden data a web presence needs, and get them into your script.
---

# Read a site

Your script never runs inside the page. The extension reads the values you name under `page`
in the manifest and hands them to `read` in `page.values`. This guide shows how to find each
kind of value. Every extractor and its fields are listed in [Extractors](09-extractors.md).

## Find a selector

1. Open the site, right-click the element and choose Inspect.
2. In the tools, right-click its line, then Copy, Copy selector.
3. Shorten what you copied. `h1.video-title` survives a redesign better than
   `#app > div:nth-child(3) > h1`.

```json
"title": { "from": "text", "selector": "h1.video-title" }
```

To check a selector before saving, type `document.querySelector("h1.video-title")` in the
browser's Console.

Sites change, so any value can arrive empty. Give each one a fallback in `read`:

```js
const title = page.values.title || page.title;
```

## Read a player inside a frame

Many sites put their player in an iframe from another site, and a selector cannot reach inside
it. `page.frames` says which frames to read and what to take from them:

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

`player` arrives in `page.values` next to `title`, so the script does not change. To find the
frame's address, right-click the player, choose Inspect and look for the `<iframe>` around it.
`presences/dailymotion` reads its player this way.

## Read several items

`list` reads every match:

```json
"genres": { "from": "list", "selector": "a.genre", "max": 3 }
```

```js
const genres = page.values.genres;   // ["Action", "Drama", "Comedy"]
const state = genres.join(", ");     // "Action, Drama, Comedy"
```

## Read data the page does not show

The text on screen is often not the best source. Sites keep cleaner data elsewhere, and a
presence can read it without running anything in the page.

| Where the data is | How to find it | Extractor |
| --- | --- | --- |
| A global variable | Type `window.` in the Console and explore. | `{ "from": "variable", "path": "app.video.episode" }` |
| JSON in a script tag | Search the page source for `__NEXT_DATA__` or `application/ld+json`. | `{ "from": "json", "selector": "script#__NEXT_DATA__", "path": "props.pageProps.title" }` |
| Saved by the site | Look in the Application tab of the browser's tools. | `{ "from": "storage", "key": "player-state", "path": "track.title" }` |
| A background picture | Inspect the element and find `background-image`. | `{ "from": "style", "selector": ".poster", "name": "background-image" }` |
| Inside a web component | Write `>>>` to cross into it. | `{ "from": "text", "selector": "video-player >>> h2" }` |
| The site's media controls | Music sites often fill them. | `{ "from": "session" }` |

A name that holds `token`, `password`, `secret`, `cookie` and the like is refused, so a
presence cannot read a login. Read only what you show: reviewers check each path.

## Start without code

Open the extension on a site and press "Make a presence". Click the texts you want and give it
a name. The application writes `matches` and `extract` for you, and you can keep editing from
there.
