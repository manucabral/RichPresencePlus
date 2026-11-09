---
title: Start here
description: Write presences for Rich Presence Plus in JavaScript, for websites and Windows programs, with no build step and no extra tools.
---

# Start here

A presence shows on Discord what someone is doing on a website or in a Windows program. It is
a folder with a manifest and one JavaScript file. Every few seconds the application asks your
script what is happening, and it takes care of Discord.

```js
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;
  return { title: page.title };
}

function activity(seen) {
  return { details: cut(seen.title), kind: "watching" };
}
```

That is a complete script. `read` collects what it needs, and `activity` says what the card
shows. You need no Rust, no Node and no build step: save the file and the application loads it.

## Where to go

| You want to | Read |
| --- | --- |
| Make your first presence, step by step | [Your first presence](02-first-presence.md) |
| Show a Windows program or a game | [Show a Windows program](04-desktop.md) |
| Get a value out of a site | [Read a site](03-read-a-site.md) |
| Fix a presence that shows nothing | [When nothing shows](06-troubleshoot.md) |
| Look up a field or a function | [Manifest](08-manifest.md), [Script](10-script.md), [Helpers](12-helpers.md) |
| Understand what happens behind the card | [How it works](14-how-it-works.md) |

## How these pages are organized

- **Start** teaches with one guided example.
- **Guides** each solve one task.
- **Reference** lists every field, function and limit, in the same shape on every page.
- **How it works** explains the design behind the rules.

## What you need

- Windows 10 or 11, with the Discord desktop app open.
- Rich Presence Plus installed.
- A code editor. Visual Studio Code also explains each function as you type.
- For websites, the browser extension, connected from the Browser page of the application.
