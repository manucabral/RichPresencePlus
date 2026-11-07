---
title: Structure
description: What each file of a Rich Presence Plus presence does, how the manifest works, and how to add options and translations.
---

# Structure

Each presence is one folder. This is everything that can be in it:

```
my-site/
  manifest.json       who the presence is and what it may read
  main.js             your code
  locales/en.json     the texts your code shows
  locales/es.json     the same texts in another language
  icon.webp           optional logo
  .vscode/            help for your editor, written by the app
  jsconfig.json       connects that help to your editor
```

Your presences live in `%LOCALAPPDATA%\com.manucabral.richpresenceplus\presences`. The
Developer page opens that folder for you.

## manifest.json

The manifest describes the presence before any of your code runs. The application uses it to
show the card, and to decide what your script may touch.

```json
{
  "name": "My Site",
  "version": "1.0.0",
  "description": "Shows the page you are on.",
  "client_id": "1567580382377803786",
  "kind": "video",
  "interval": 5,
  "entry": "main.js",
  "domains": ["mysite.com"],
  "page": {
    "matches": ["*://www.mysite.com/*"],
    "extract": {
      "title": { "from": "text", "selector": "h1" }
    }
  }
}
```

| Field | What it means |
| --- | --- |
| `name` | The name people see. It must be unique. |
| `version` | Raise it each time you change a published presence. |
| `description` | One line for the card. |
| `client_id` | The Discord application. The template uses one shared by every presence. |
| `kind` | `game`, `video`, `music` or `other`. It sets which presence wins when several run at once. |
| `interval` | Seconds between two calls to your script. |
| `entry` | The file with your code. |
| `domains` | The sites involved, so people can exclude them in Privacy. |
| `page` | What to read from websites. See [Web Presences](05-web.md). |
| `permissions` | What to read from the computer. See [Desktop Presences](06-desktop.md). |

Your script starts with access to nothing. It can only reach what the manifest asks for, and
people see that list before they turn the presence on.

Optional fields fill in the card: `author`, `contributors`, `color`, `tags` and `icon`. The
Code tab of the Developer page has a form for them.

Tags come from a fixed list, so people find every presence of a type with one word. Choose up
to 8: `video`, `streaming`, `movie`, `series`, `anime`, `live`, `sport`, `music`, `radio`,
`podcast`, `game`, `multiplayer`, `reading`, `manga`, `news`, `wiki`, `blog`, `code`, `design`,
`study`, `social`, `chat`, `forum`, `shopping`, `ai` and `tool`.

## main.js

This file holds your functions, which the next page explains. It is one file: a script
cannot load other files or use `import`.

Open the presence's folder in Visual Studio Code and the editor explains what you are using. The
application writes the files for that, and they follow your manifest:

- `page.values.` offers the values of your `extract`, and shows which selector each one comes from.
- `context.options.` offers your options, and a `choice` lists its values.
- `host.` offers only what your `permissions` give, and marks anything else as an error.
- Hovering `playback`, `cut` or any field of `activity` explains it, with an example.
- `manifest.json` autocompletes and warns about a wrong field.

So the editor knows what each function receives, the application adds one comment above `read`,
`activity` and `meta`. It changes no code, and these files never reach the catalog.

## locales

The script does not write its texts directly. It asks for them by key, so a presence can
speak several languages without changing the code.

`locales/en.json` holds every text:

```json
{
  "description": "Shows the page you are on.",
  "browsing": "Browsing",
  "by": "By {name}"
}
```

The script uses `t`:

```js
t("browsing");               // "Browsing"
t("by", { name: "Manu" });   // "By Manu"
```

To add a language, copy the file to `locales/es.json` and translate the values. Keep the keys
and the `{name}` parts as they are. A key you did not translate shows in English.

## Options

Options let each person change how the presence behaves. Declare them in the manifest:

```json
"options": [
  { "type": "boolean", "key": "show_title", "default": true }
]
```

The application draws the control on the card of the presence. Your code reads the value:

```js
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;
  return { title: context.options.show_title ? page.title : "A private page" };
}
```

The label comes from the translations, with the key `option.show_title`.

| `type` | Fields |
| --- | --- |
| `boolean` | `default` |
| `number` | `default`, optional `min` and `max` |
| `text` | `default`, optional `max_length` |
| `choice` | `default`, `choices` |

Each choice takes its label from `option.<key>.<choice>`. Adding `"when": "<key>"` shows an
option only while that other boolean option is on.
