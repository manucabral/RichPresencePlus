# Translating

Texts live in JSON files in this repository. A translation is a pull request
that adds or edits one file; no code changes are needed.

## Where the files are

| What | Folder | Used by |
| --- | --- | --- |
| The application window | `frontend/src/locales/` | the interface |
| One presence | `presences/<name>/locales/` | the presence script and its card |

Each file is named after a language tag: `es.json`, `pt-BR.json`, `de.json`.
`en.json` is the source; every other file may be incomplete, and a missing key
shows the English text.

## File format

One flat object. Keys are fixed; only the values are translated.

```json
{
  "dashboard.pause": "Pausar publicación",
  "dashboard.connected_one": "{count} conectado",
  "dashboard.connected_other": "{count} conectados"
}
```

- `{name}` marks a value filled in by the program. Keep every placeholder the
  English text has, spelled the same.
- Keys ending in `_one` and `_other` are plural forms. Add the forms your
  language uses (`_zero`, `_two`, `_few`, `_many`) next to `_other`.
- A presence's `description` key replaces the description shown on its card.
- Texts shown on Discord have limits (128 characters for most fields). Keep
  translations about as short as the English.

## Adding a language

1. Copy `en.json` to `<tag>.json` in the folder you are translating.
2. Translate the values. You can leave a key out; English fills the gap.
3. Run `node tools/check-locales.mjs`. It lists wrong tags, keys English does
   not have, changed placeholders and the share of keys each file covers.
4. Open a pull request. CI runs the same check.

Languages appear in the language menu of the application as soon as the file
is part of a release. There is no list to edit.

## Translating a presence you wrote

Presence scripts call `t(key, values)`:

```js
details: t("by", { author: snapshot.author })
```

Put the English texts in `locales/en.json` and use the same keys in every
other file. The check fails if the script uses a key English does not define.
Brand names, such as the name of a service, are not translated.

## What is not translated

Error messages from the program itself (a file that cannot be read, a failed
connection) stay in English so that reports from any language read the same.
