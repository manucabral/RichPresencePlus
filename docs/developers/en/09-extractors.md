---
title: Extractors
description: Every way a web presence can read a page, with its fields and what it gives your script.
---

# Extractors

An extractor reads one value from a page. Each entry of `page.extract` or `page.frames.extract`
is an extractor, and its value reaches `read` in `page.values` under the same name. A value
that cannot be read arrives as `null`. Texts arrive with their spaces joined, up to 512
characters, and a list or object from the page up to 4 KB.

```json
"extract": {
  "title": { "from": "text", "selector": "h1" },
  "player": { "from": "media", "selector": "video" }
}
```

## Summary

| `from` | Fields | Gives |
| --- | --- | --- |
| `text` | `selector` | Text |
| `attr` | `selector`, `name` | Text |
| `list` | `selector`, optional `name` and `max` | List of texts |
| `meta` | `name` | Text |
| `media` | optional `selector` | `{ paused, current, duration, rate }` |
| `session` | | `{ title, artist, album, artwork, state }` |
| `variable` | `path` | Text, number, boolean, or a small list or object |
| `json` | `selector`, optional `path` | Same as `variable` |
| `storage` | `key`, optional `area` and `path` | Same as `variable` |
| `style` | `selector`, `name` | Text |
| `response` | `url`, optional `path` | Same as `variable` |

A selector is CSS, up to 256 characters. Write `>>>` to cross into a web component, as in
`video-player >>> h2`. A path is names joined by dots, such as `video.title`: up to 128
characters and 8 names. It can only read, never call.

## text

The visible text of the first element that matches `selector`.

```json
"title": { "from": "text", "selector": "h1.video-title" }
```

## attr

The attribute `name` of the first element that matches `selector`. `href` and `src` arrive as
full addresses, even when the page writes them relative.

```json
"channel_url": { "from": "attr", "selector": "a.channel-name", "name": "href" }
```

## list

Every element that matches, as a list of texts, or of the attribute `name`. `max` limits it: 16
by default, at most 32.

```json
"genres": { "from": "list", "selector": "a.genre", "max": 3 }
```

## meta

The content of the `<meta>` tag whose `name` or `property` is `name`.

```json
"image": { "from": "meta", "name": "og:image" }
```

## media

The state of a `<video>` or `<audio>` that matches `selector`, or of any of them without one.
When several match, it picks one that plays with sound, then one with sound, then one that
plays. `current` and `duration` are seconds; `duration` is `null` while unknown.
Pass it to [`playback`](12-helpers.md#playbackplayer-receivedat) for the progress bar.

```json
"player": { "from": "media", "selector": "video" }
```

## session

What the site gives the browser's media controls: `title`, `artist`, `album`, `artwork` (an
address) and `state`. Music sites usually fill it, so a presence may need no selector at all.

```json
"song": { "from": "session" }
```

## variable

What the page keeps in a global variable, by its `path`.

```json
"episode": { "from": "variable", "path": "app.video.episode" }
```

## json

JSON written in the element that matches `selector`, such as `script#__NEXT_DATA__`. With
`path`, only that part.

```json
"title": { "from": "json", "selector": "script#__NEXT_DATA__", "path": "props.pageProps.title" }
```

## storage

An entry of `localStorage` by its `key`, or of `sessionStorage` with `"area": "session"`. With
`path`, only that part of its JSON.

```json
"song": { "from": "storage", "key": "player-state", "path": "track.title" }
```

## style

A computed CSS property of the first element that matches `selector`, by its `name`.

```json
"cover": { "from": "style", "selector": ".poster", "name": "background-image" }
```

## response

The last JSON answer the page received from an address that matches `url`, whether it asked
with `fetch` or `XMLHttpRequest`. With `path`, only that part. `url` is a match pattern that
names a site, such as `*://api.example.com/v1/player/*`, or a path that starts with `/` on the
site of the page itself; its path also matches the query.

```json
"title": { "from": "response", "url": "*://api.example.com/v1/player/*", "path": "data.title" }
```

The page gets its answer as before; the extension only looks at a copy. Answers of more than
1 MB are left alone. An answer that arrives before the extension starts on the page is still
read, and a tab that was open before the presence was turned on shows only the answers that
come after. A presence reads up to 8 addresses, and only in its pages, not in frames.

To find the address, open the Network tab of the browser's tools, filter by Fetch/XHR, and play
or open what the presence shows.

## What cannot be read

A path or key that holds `token`, `password`, `secret`, `cookie` and the like is refused when
the presence loads, so a presence cannot read a login. Neither can an address that holds those
words, `login`, `oauth` or `auth/`. A frame is read only inside the pages of
its own presence.
