---
title: Limits and errors
description: The limits every presence runs within, and what each error message of the application means.
---

# Limits and errors

## Limits of a script

| What | Limit |
| --- | --- |
| Memory | 16 MiB |
| Loading the file | 1 second |
| Each call | 2 seconds |
| What `read` returns | 64 KiB |
| `interval` | 1 to 300 seconds |
| Console | 200 lines kept, 50 per call, 500 characters per line |

Variables persist between calls, so a cache is fine, but limit how much it keeps. A script that
goes over a limit stops for that round and the Test tab shows the error.

## Limits of a manifest

| What | Limit |
| --- | --- |
| `page.matches` | 1 to 16 patterns |
| `page.frames.matches` | 1 to 8 patterns |
| Values in one `extract` | 32 |
| A selector, attribute or meta name | 256 characters |
| A path | 128 characters, 8 names |
| `list` items | 16 by default, at most 32 |
| `options` | 16, with up to 16 choices each |
| `domains` | 32 |
| `args` | 16 programs, 1 to 16 options each |
| `files.tail` | 500 lines |
| `tags` | 8 |

## Limits of the card

| What | Limit |
| --- | --- |
| Each text | 2 to 128 characters |
| An address or image | 512 characters, `http://` or `https://`, no user or password |
| Buttons | 2, with labels of 1 to 32 characters |
| `party_size` | 1 to 1000 |

## Messages of the script

| Message | What it means |
| --- | --- |
| `script failed to load: ...` | The file has a syntax error, or failed while loading. |
| `script must define the function read` | `read` or `activity` is missing or misspelled. |
| `script error: ...` | A function threw. The rest of the message is the error. |
| `script exceeded its time budget` | A call took more than 2 seconds. Look for a loop that never ends. |
| `script returned invalid data: ...` | A function returned something that is not data. `read must not return a promise` means it was `async`, which is not allowed. |
| `snapshot exceeds 65536 bytes` | `read` returned too much. Keep only what `activity` uses. |
| `permission denied: ...` | The script called a `host` function the manifest does not ask for. |
| `code generation is disabled` | The script tried `new Function`. Building code from text is not allowed, and `eval` does not exist. |

## Messages of the activity

| Message | What it means |
| --- | --- |
| `details must have at most 128 characters` | A text is too long. Use `cut`. |
| `use at most 2 buttons` | `buttons` has more than two. |
| `button label must have 1 to 32 characters` | A button label is empty or too long. |
| `button address must be HTTP or HTTPS without credentials` | A button `url` is not a web address. |
| `end must not be earlier than start` | `end` is before `start`. |
| `details_url must be an HTTP or HTTPS address without credentials` | A link is not a web address. |
| `party_size must be [current, largest] with 1 <= current <= largest <= 1000` | The party numbers are out of range. |

The first column shows one field; the same message names whichever field broke the rule.

## Messages of the manifest

A manifest error names the field and what it expects, such as `interval: expected 1 to 300
seconds` or `color: expected #RRGGBB`. Every field and its limits are in [Manifest](08-manifest.md).
