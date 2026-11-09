---
title: Manifest
description: Every field of manifest.json, with its type, whether it is required, its default value and its limits.
---

# Manifest

`manifest.json` describes a presence before any of its code runs. The application uses it to
draw the card, to decide what the script may touch and to check updates. A field it does not
know is an error.

```json
{
  "name": "My Video",
  "version": "1.0.0",
  "description": "Shows the video you are watching.",
  "client_id": "1567580382377803786",
  "kind": "video",
  "interval": 5,
  "domains": ["manucabral.github.io"],
  "page": {
    "matches": ["*://manucabral.github.io/*practice/*"],
    "extract": { "title": { "from": "text", "selector": "h1.video-title" } }
  },
  "author": "manucabral",
  "tags": ["video"]
}
```

## Identity

| Field | Type | Required | Default | What it does |
| --- | --- | --- | --- | --- |
| `name` | text | yes | | The name people see. Letters, digits, spaces, `_`, `-` and `.`. It must be unique. |
| `version` | text | yes | | Up to 32 characters, such as `1.2.0`. Raise it each time you publish a change. |
| `description` | text | no | | One line for the card. `description` in a locale file replaces it in that language. |
| `client_id` | text | yes | | The Discord application, 17 to 20 digits. `1567580382377803786` is shared by every presence. |
| `author` | text | no | | Who wrote it, up to 64 characters. |
| `contributors` | list of texts | no | `[]` | Up to 10 people, 64 characters each. |
| `color` | text | no | | Color of the card, as `#RRGGBB`. |
| `icon` | text | no | | A `.png` or `.webp` file of the folder, or an `https://` address, up to 512 characters. |
| `tags` | list of texts | no | `[]` | Up to 8 words from the list below. Words the application does not know are left out. |

Tags: `video`, `streaming`, `movie`, `series`, `anime`, `live`, `sport`, `music`, `radio`,
`podcast`, `game`, `multiplayer`, `reading`, `manga`, `news`, `wiki`, `blog`, `code`, `design`,
`study`, `social`, `chat`, `forum`, `shopping`, `ai` and `tool`.

## Behavior

| Field | Type | Required | Default | What it does |
| --- | --- | --- | --- | --- |
| `kind` | `game`, `video`, `music` or `other` | no | from the activity | Sets the default priority when several presences run at once. |
| `interval` | number | no | `15` | Seconds between two calls to the script, from 1 to 300. |
| `hide_when_paused` | boolean | no | `true` | Hides the card while `meta` says `paused`. People can change it. |
| `entry` | text | no | `main.js` | The script file, a relative `.js` path inside the folder. |
| `domains` | list of texts | no | `[]` | Up to 32 lower-case host names, so people can exclude them in Privacy. |

`kind` in the manifest is not `kind` in the activity. The manifest's sets the priority:
`game` 80, `video` 60, `music` 40, `other` 20. Without it, the activity's `kind` decides:
`playing` and `competing` count as `game`, `listening` as `music`, `watching` as `video`.

## Access

| Field | Type | Required | Default | What it does |
| --- | --- | --- | --- | --- |
| `page` | object | no | | What the extension reads from websites. See [Extractors](09-extractors.md). |
| `permissions` | object | no | none | What the script may read from the computer. See [Host](11-host.md). |

A script starts with access to nothing. People see these two fields before they turn the
presence on, and an update that asks for more waits until they accept it.

### page

| Field | Type | Required | What it does |
| --- | --- | --- | --- |
| `matches` | list of patterns | yes | Addresses the presence reads, 1 to 16, such as `*://www.example.com/*`. |
| `extract` | object | no | Up to 32 values, each under a name of letters, digits and `_`. |
| `frames` | object | no | Frames of other sites to read inside those pages: `matches` (1 to 8) and `extract`. |

A pattern starts with `*://`, `http://` or `https://`, names a host and has a path. `*` matches
anything.

## Options

`options` is a list of up to 16 settings people can change. The script reads them in
`context.options`, and each label comes from the locale key `option.<key>`.

| Field | Type | Required | What it does |
| --- | --- | --- | --- |
| `type` | `boolean`, `number`, `text` or `choice` | yes | Which control the application draws. |
| `key` | text | yes | The name in `context.options`. Unique in the presence. |
| `default` | depends on `type` | yes | The value before anyone changes it. |
| `when` | text | no | The key of a boolean option; this one shows only while that one is on. |
| `min`, `max` | number | no | Limits of a `number`. |
| `max_length` | number | no | Longest `text`: 64 by default, at most 256. |
| `choices` | list of texts | for `choice` | Up to 16 values. Each label comes from `option.<key>.<choice>`. |

`when` cannot name the option itself or form a loop. How to use options:
[Options and languages](05-options.md).
