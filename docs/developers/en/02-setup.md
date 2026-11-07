---
title: Setup Instructions
description: Set up what you need to write a Rich Presence Plus presence, create one from a template and watch it change on every save.
---

# Setup Instructions

By the end of this page you will have a presence running, and you will see it change each
time you save the file.

## Prerequisites

- Windows 10 or 11, with the Discord desktop app open.
- Rich Presence Plus installed.
- A code editor, such as Visual Studio Code.
- For web presences, the browser extension connected. The Browser page of the application
  explains how.

## Create a presence

1. Open the application and go to the Developer page.
2. Under "Create a presence", type `My Site`.
3. Leave the template on "Web page" and press Create.

The application writes a folder and moves to the Code tab. Press "Open in VS Code", or "Open
folder" if you use another editor. You will find a working presence: the next pages explain
each file in it.

## Choose the name on Discord

The template already works on Discord. It uses an application shared by every presence, and
its script sets the title with `name`:

```js
function activity(site) {
  return { name: "My Site", details: cut(site.title), kind: "watching" };
}
```

You do not need to create anything on Discord. Create your own application at
discord.com/developers only if you want your own images: anything you upload under Rich
Presence, Art Assets can be used by name in `large_image`. Then paste its Application ID in
`client_id`.

## Watch it live

This is how you will work from now on.

1. In the Developer page, open the Test tab, pick My Site and press Watch.
2. Open `main.js`, change something and save.

The application loads the new version in less than a second. The page runs your script and
shows what `read` returned, the activity, the lines of `console.log` and a preview of the
card as Discord draws it.

If you save a mistake, a red message says what failed and the last version that worked keeps
running. Fix it and save again. Nothing you write here can break the application.

To delete a presence of yours, open the Code tab and press "Delete presence". It goes to the
Windows Recycle Bin, so you can get it back from there. You can also add or delete folders in
File Explorer: the application notices within a few seconds.

## When something does not show

| What you see | Why | What to do |
| --- | --- | --- |
| The preview is empty | The extension cannot read the site, or `matches` does not fit the address. | Check `matches`, then open the extension and allow the site. |
| A red "permission" error | The script calls something the manifest does not ask for. | Add it under `permissions`. |
| The activity disappears | `read` returned `null`, or it failed. | Add `console.log` and read the Console block. |
| A text shows as its key | The key is missing from `locales/en.json`. | Add it there. |
| Discord shows another name | `activity` returns no `name`. | Add `name` to what it returns. |
| A save does nothing | The presence came with the app or from the catalog. | Copy it in the Developer page, then edit the copy. |
