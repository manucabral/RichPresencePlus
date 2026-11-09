---
title: Helpers
description: The functions every presence script can call with nothing to import, with what each one gives back.
---

# Helpers

Every script can call these functions, with nothing to import or ask for.

| Helper | Gives |
| --- | --- |
| `t(key, values)` | The text of `key` in the person's language. |
| `cut(text, max)` | A text short enough for Discord. |
| `parseUrl(address)` | The parts of an address. |
| `bestPage(pages)` | The page that matters when several match. |
| `playback(player, receivedAt)` | The playing flag and the progress bar of a video. |
| `parseTime(text)` | A clock text as seconds. |
| `progress(current, duration, now)` | The progress bar for a position and a length. |
| `console.log(...)` | A line in the Console block of the Developer page. |

## t(key, values)

The text of `key` from `locales/<language>.json`, with each `{name}` filled from `values`.

```js
t("by", { name: "Manu" });   // "By Manu"
```

A key missing in the person's language comes from `en.json`. A key missing there too gives the
key itself. See [Options and languages](05-options.md#write-texts-by-key).

## cut(text, max)

Shortens a text to `max` characters, 128 by default, which is Discord's limit.

```js
cut("A very long title", 10);   // "A very lo…"
```

**Returns** `null` when `text` is not a text.

## parseUrl(address)

Splits an address. A presence has no `URL` class.

```js
parseUrl("https://a.com/watch?v=1");   // { host: "a.com", path: "/watch", query: { v: "1" } }
```

**Returns** `null` when it is not an address.

## bestPage(pages)

Chooses when several tabs match: the one playing, then one with a player, then the first. It
looks for the value named `player`, so use that name for a `media` extractor.

```js
const page = bestPage(context.pages);
if (!page) return null;
```

**Returns** `null` when `pages` is empty.

## playback(player, receivedAt)

Turns the value of a `media` extractor into `{ playing, start, end }`. Pass `page.received_at`
as `receivedAt`, so the bar counts from when the page was read.

```js
return { title, ...playback(page.values.player, page.received_at) };
```

While paused, `playing` is `false` and `start` and `end` are `null`, so the bar goes away by
itself. When the length is unknown, only `end` is `null` and the time counts up. The speed of
the player counts: at 2x, a 4-minute video ends in 2 minutes.

## parseTime(text)

Reads `1:02:03`, `4:05` or `30` as seconds.

```js
parseTime("1:02:03");   // 3723
```

**Returns** `null` when the text is not a time.

## progress(current, duration, now)

The `{ start, end }` that draw a bar at `current` seconds of `duration`, for `context.now`.
Useful when a site shows the time as text instead of a player.

```js
const bar = progress(parseTime("4:05"), parseTime("10:00"), context.now);
```

## console.log(...)

Writes a line in the Console block of the Developer page. Objects are written as JSON.
`console.warn` and `console.error` work the same and mark the line. The page keeps the last
200 lines, 50 per call, up to 500 characters each.
