---
title: Options and languages
description: Let people change how a presence behaves with options, and show its texts in several languages.
---

# Options and languages

## Add an option

Options let each person change how the presence behaves. The application draws the control on
the presence's card, and your code reads the value.

1. Declare it in the manifest:

   ```json
   "options": [
     { "type": "boolean", "key": "show_title", "default": true }
   ]
   ```

2. Give it a label in `locales/en.json`, under `option.` and its key:

   ```json
   "option.show_title": "Show the video title"
   ```

3. Read it in `read`:

   ```js
   function read(context) {
     const page = bestPage(context.pages);
     if (!page) return null;
     return { title: context.options.show_title ? page.title : t("private") };
   }
   ```

Save and open the Presences page: the switch is on the presence's card. The other types,
`number`, `text` and `choice`, are in [Manifest](08-manifest.md#options).

## Show an option only when another is on

Add `when` with the key of a boolean option:

```json
"options": [
  { "type": "boolean", "key": "show_title", "default": true },
  { "type": "number", "key": "title_length", "default": 64, "min": 16, "max": 128, "when": "show_title" }
]
```

`title_length` hides while `show_title` is off. Your script still receives its value.

## Offer a list of choices

```json
{ "type": "choice", "key": "line", "default": "channel", "choices": ["channel", "views"] }
```

Each choice takes its label from `option.<key>.<choice>`:

```json
"option.line": "Second line",
"option.line.channel": "Channel",
"option.line.views": "Views"
```

## Write texts by key

The script does not write its texts directly. It asks for them by key with `t`, so the same
code speaks every language the folder has.

```json
{
  "description": "Shows the video you are watching.",
  "by": "By {name}"
}
```

```js
t("by", { name: "Manu" });   // "By Manu"
```

`description` is also the description of the presence in that language.

## Add a language

1. Copy `locales/en.json` to `locales/es.json`.
2. Translate the values. Keep the keys and the `{name}` parts as they are.

The application picks the file of the person's language. A key missing there shows in English,
and a key missing in English shows as the key itself.
