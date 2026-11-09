---
title: Your first presence
description: Build a working presence for a video page in about ten minutes, and watch it update on every save.
---

# Your first presence

In this tutorial you build a presence for a video page. When it is done, the card shows the
video's title and channel, two buttons and a progress bar that stops when you pause.

You will use a practice page made for this tutorial. It never changes, so every step works as
written: [manucabral.github.io/RichPresencePlus/practice](https://manucabral.github.io/RichPresencePlus/practice/).

## 1. Create the folder

1. Open the application and go to the Developer page.
2. Under "Create a presence", type `My Video`.
3. Leave the template on "Web page" and press Create.

The application writes a working presence and opens the Code tab. Press "Open in VS Code", or
"Open folder" if you use another editor.

## 2. Watch it live

1. Open the Test tab, pick My Video and press Watch.
2. Leave that window where you can see it.

From now on, every time you save a file the application loads the new version in less than a
second. The Test tab shows what `read` returned, the activity, your `console.log` lines and a
preview of the card.

## 3. Say which page to read

Open `manifest.json` and replace the `page` and `domains` entries with these:

```json
"domains": ["manucabral.github.io"],
"page": {
  "matches": ["*://manucabral.github.io/*practice/*"],
  "extract": {
    "title": { "from": "text", "selector": "h1.video-title" },
    "channel": { "from": "text", "selector": "a.channel-name" },
    "channel_url": { "from": "attr", "selector": "a.channel-name", "name": "href" },
    "player": { "from": "media", "selector": "video" }
  }
}
```

Also change `"kind": "other"` to `"kind": "video"`, delete the `options` entry, which this
presence does not use, and save.

`matches` says which addresses belong to this presence. Each entry of `extract` reads one value
from the page, under a name you choose: a text, an attribute and the state of the video.

Now open the practice page in your browser. The extension asks before reading a new site: open
the extension and press "Allow this site".

The Test tab now shows the four values under `read`. Press play on the practice page and watch
`player` change.

## 4. Collect what you need

Replace all of `main.js` with this:

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

Save. The Test tab shows the object, with three new fields: `playing`, `start` and `end`.

`bestPage` picks the right tab when several match. `playback` turns the video's state into
those three fields, and `...` adds them to the object. When the page is closed, `read` returns
`null` and the card goes away.

## 5. Show it on the card

Add this below `read`:

```js
function activity(video) {
  const buttons = [{ label: t("watch"), url: video.url }];
  if (video.channel_url) buttons.push({ label: t("channel"), url: video.channel_url });

  return {
    name: "My Video",
    details: cut(video.title),
    state: video.channel || t("watching"),
    kind: "watching",
    start: video.start,
    end: video.end,
    buttons,
  };
}
```

Then open `locales/en.json` and replace it with the texts the script asks for:

```json
{
  "description": "Shows the video you are watching.",
  "watch": "Watch",
  "channel": "Channel",
  "watching": "Watching"
}
```

Save both files. The preview shows the name, the title, the channel, a progress bar and two
buttons. `name` replaces the name of the shared Discord application. With
Discord open, your profile shows the same card.

## 6. Give way when paused

Add one more function:

```js
function meta(video) {
  return { paused: !video.playing };
}
```

Save and pause the video. The progress bar goes away, because `start` and `end` are empty while
paused, and the card hides on the next round. If music plays somewhere else, that presence takes
the card meanwhile.

## What you built

`main.js` now has three functions, and the application calls them in order: `read` collects,
`activity` presents, `meta` reports. This is the whole file:

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

function activity(video) {
  const buttons = [{ label: t("watch"), url: video.url }];
  if (video.channel_url) buttons.push({ label: t("channel"), url: video.channel_url });

  return {
    name: "My Video",
    details: cut(video.title),
    state: video.channel || t("watching"),
    kind: "watching",
    start: video.start,
    end: video.end,
    buttons,
  };
}

function meta(video) {
  return { paused: !video.playing };
}
```

To do the same on a real site, change `matches` and the selectors: [Read a site](03-read-a-site.md)
shows how to find them. `presences/youtube` in the repository is this same presence for a real
site.
