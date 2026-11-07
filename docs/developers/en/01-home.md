---
title: Home
description: Learn how to write presences for Rich Presence Plus in JavaScript, for websites and Windows programs, with no build step and no extra tools.
---

# Home

This documentation teaches you how to write a presence for Rich Presence Plus, the app that
shows on Discord what you do on a website or in a Windows program.

A presence is a small folder with one JavaScript file. Its job is to answer one question
over and over: what is this person doing right now? The application asks every few seconds,
your code answers, and the application takes care of Discord.

Here is a complete script. It shows the title of the page you are on:

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

`read` looks at the page and keeps what it needs. `activity` turns that into what Discord
shows. Those two functions are most of what you will write.

You need no Rust, no Node and no build step. You save the file and the application loads it.

## Getting started

- [Setup Instructions](02-setup.md)
- [Structure](03-structure.md)
- [Functions](04-functions.md)
- [Web Presences](05-web.md)
- [Desktop Presences](06-desktop.md)
- [Publishing](07-publish.md)
