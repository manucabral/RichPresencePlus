---
title: Desktop Presences
description: Build a presence for a Windows program or game step by step, using permissions to read running programs, windows, log files and web APIs.
---

# Desktop Presences

A desktop presence shows what you do in a Windows program. It reads the computer through the
`host` object, which starts empty. Each permission in the manifest adds a function to it, and
people see that list before they turn the presence on.

This page builds a presence for a game, one piece at a time.

## 1. Ask for what you need

To know whether a game is open, the presence needs the list of running programs:

```json
"permissions": { "process": true }
```

Now `host.process.list()` exists. Without the permission, calling it fails and the Developer
page shows the error.

## 2. Find the program

```js
const PROGRAM = "game.exe";

function read(context) {
  const running = host.process.list().some((name) => name.toLowerCase() === PROGRAM);
  if (!running) return null;
  return { playing: true };
}
```

Program names can come in any case, so the script compares them in lower case. Returning `null`
when the game is closed is what makes the presence go away. To see a program's exact name, open
the Details tab in Task Manager.

## 3. Count the time

Discord draws the counter from `start`, so the script has to remember when the game opened. A
variable outside the functions survives between calls.

```js
let since = 0;

function read(context) {
  const running = host.process.list().some((name) => name.toLowerCase() === PROGRAM);
  if (!running) {
    since = 0;
    return null;
  }
  if (since === 0) since = context.now;
  return { since };
}
```

## 4. Show it

```js
function activity(game) {
  return {
    details: "My Game",
    state: t("playing"),
    kind: "playing",
    start: game.since,
    large_image: "logo",
  };
}

function meta() {
  return { application: PROGRAM };
}
```

`"logo"` is the name of an image uploaded to your Discord application. `meta` gives the program's
name so people can exclude it in Privacy.

## 5. Know when it is in front

Ask for the `window` permission too, and the game can rank above other presences while you are
playing it:

```json
"permissions": { "process": true, "window": true }
```

```js
function read(context) {
  // ...the code above...
  const window = host.window.foreground();
  return { since, focused: Boolean(window) && window.process.toLowerCase() === PROGRAM };
}

function meta(game) {
  return { application: PROGRAM, foreground: game.focused };
}
```

`foreground` also gives the window's `title`, which often holds a file or project name. If the
program's name is enough, leave the title alone.

## 6. Read more about the game

Log files often say what is happening inside a program. This reads the end of the newest log:

```json
"permissions": { "process": true, "files": ["%LOCALAPPDATA%/MyGame/logs"] }
```

```js
function read(context) {
  // ...the code above...
  const log = host.files.latest(host.files.roots[0], ".log");
  const lines = host.files.tail(log, 50);
  const level = lines.map((line) => /Entered level (.+)/.exec(line)).findLast(Boolean);
  return { since, level: level ? level[1] : null };
}
```

Then show it with `state: game.level || t("playing")` in `activity`.

Read the end of a file and not all of it, and keep the path of the log for a minute instead of
searching on every call: a folder can hold hundreds of files.

## All the permissions

| Permission | What it adds |
| --- | --- |
| `"process": true` | `host.process.list()`: names of the running programs. |
| `"args": {"game.exe": ["-h", "-p"]}` | `host.process.args(name)`: only those options of its command line, or `null` if it is closed. |
| `"window": true` | `host.window.foreground()`: `{ process, title }` of the focused window. |
| `"media": true` | `host.media.current()`: what Windows says is playing. |
| `"query": ["samp"]` | `host.query(protocol, address, port)`: what a game server says about itself, or `null`. |
| `"http": ["api.example.com"]` | `host.http.get(url)`: HTTPS only, to those exact hosts, no redirects. |
| `"files": ["%LOCALAPPDATA%/Game/logs"]` | `host.files.latest(dir, suffix)` and `host.files.tail(path, lines)` inside those folders. |

Ask only for what the script uses. An update that adds a permission waits until the person
accepts it, and the installed version keeps running meanwhile.

## The real thing

`presences/ready-or-not` is steps 1 to 4 for a real game. `presences/roblox` goes further with a
log file and a web API, and `presences/creator-tools` looks only at the program in front. All
three are in the repository.

## Without code

To show a program with no code, open the Presences page and use "Add a program". Pick it from
the list, write the lines and the application makes the folder for you.
