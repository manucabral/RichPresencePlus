---
title: When nothing shows
description: Find out why a presence shows nothing or the wrong thing, from the Test tab to the messages the application gives.
---

# When nothing shows

Open the Developer page, pick the presence in the Test tab and press Watch. Most problems show
there: what `read` returned, the activity, the console and any error in red.

## Find where it stops

Work from the start of the chain to the end:

1. **The page.** Under `read`, are there any pages? If not, the extension is not reading the site.
2. **The values.** Do the pages have the values you asked for? If one is empty, its selector or
   path does not match.
3. **`read`.** Does it return an object? `null` means nothing shows, on purpose.
4. **`activity`.** Does the preview show the card? A red error names the field that broke a limit.
5. **Discord.** Does your profile show it? If the preview is right and Discord is not, another
   presence may have a higher priority, or a privacy rule hides it.

Add `console.log(...)` anywhere in the script and read the lines in the Console block.

## Common causes

| What you see | Why | What to do |
| --- | --- | --- |
| No pages under `read` | `matches` does not fit the address, or the site has no permission. | Check `matches`, then open the extension and press "Allow this site". |
| A value is always empty | The selector or the path does not match the page. | Test it in the browser's Console with `document.querySelector`. |
| A player inside a frame reads nothing | The frame is from another site. | Read it with `page.frames`: see [Read a site](03-read-a-site.md#read-a-player-inside-a-frame). |
| A red "permission" error | The script calls something the manifest does not ask for. | Add it under `permissions`. |
| The card disappears | `read` returned `null`, or it failed. | Log what `read` sees. |
| The card hides while paused | `meta` says `paused`. | Set `"hide_when_paused": false` to keep it. |
| A text shows as its key | The key is missing from `locales/en.json`. | Add it there. |
| Discord shows another name | `activity` returns no `name`. | Add `name`. |
| A line of text is missing | It had fewer than 2 characters. | Discord refuses those, so the application drops them. |
| A save does nothing | The presence came with the application or from the catalog. | Copy it in the Developer page, then edit the copy. |

Every error message, and what it means, is in [Limits and errors](13-limits.md).

## When the preview is right but Discord is not

- The Discord desktop app must be open. The web version does not show activities.
- The Dashboard says which account receives the activity.
- When several presences run at once, the one with the highest priority wins. The Presences
  page shows and changes each priority.
- A rule in Privacy can hide a site, a program or a field.
