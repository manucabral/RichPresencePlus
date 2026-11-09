---
title: Host
description: The permissions a presence can ask for, and the host functions each one adds to the script.
---

# Host

`host` is how a script reads the computer. It starts empty, and each permission in the
manifest's `permissions` adds functions to it. Calling one that was not asked for fails with
`permission denied` and the name of the permission.

```json
"permissions": { "process": true, "window": true, "files": ["%LOCALAPPDATA%/MyGame/logs"] }
```

| Permission | Value | Adds |
| --- | --- | --- |
| `process` | `true` | `host.process.list()` |
| `args` | object | `host.process.args(name)` |
| `window` | `true` | `host.window.foreground()` |
| `media` | `true` | `host.media.current()` |
| `files` | list of folders | `host.files` |
| `http` | list of hosts | `host.http.get(url)` |
| `query` | list of protocols | `host.query(protocol, address, port)` |

People see this list before they turn the presence on. An update that adds a permission waits
until they accept it, and the installed version keeps running meanwhile.

## process

`host.process.list()` gives the names of the running programs, such as `["explorer.exe",
"Game.exe"]`. Names keep the case Windows gives them, so compare them in lower case.

```js
const running = host.process.list().some((name) => name.toLowerCase() === "game.exe");
```

## args

`host.process.args(name)` reads chosen options from the command line of a running program.
The manifest names the program and up to 16 options for each of up to 16 programs. The script
never sees the rest of the line.

```json
"permissions": { "args": { "gta_sa.exe": ["-h", "-p"] } }
```

```js
host.process.args("gta_sa.exe");   // { "-h": "127.0.0.1", "-p": "7777" }
```

**Returns** an object with each option found: its value, or `true` when it has none. `null`
when the program is not running.

## window

`host.window.foreground()` gives `{ process, title }` of the window in front, or `null`. The
title often holds a file or project name: if the program's name is enough, leave it alone.

## media

`host.media.current()` gives what Windows says is playing: `{ app, title, artist, album,
status, position, duration }`, or `null`. `position` and `duration` are seconds.

## files

Reads log files inside the folders the manifest lists. A folder may start with an environment
variable such as `%LOCALAPPDATA%`; one that does not exist on the computer is left out.

| Member | What it does |
| --- | --- |
| `host.files.roots` | The folders the manifest lists, as paths of this computer. |
| `host.files.latest(dir, suffix)` | The newest file of `dir` whose name ends in `suffix`, or `null`. |
| `host.files.tail(path, lines)` | The last `lines` lines of a file, up to 500. |

```js
const log = host.files.latest(host.files.roots[0], ".log");
const lines = host.files.tail(log, 50);
```

Read the end of a file and not all of it, and keep the path for a minute instead of searching
on every call: a folder can hold hundreds of files.

## http

`host.http.get(url)` fetches an `https://` address of the hosts the manifest lists, exactly as
written: `api.example.com` does not allow `www.example.com`.

```json
"permissions": { "http": ["api.example.com"] }
```

**Returns** `{ status, body }`, where `body` is text. It follows no redirects, waits up to 5
seconds and reads up to 1 MB.

## query

`host.query(protocol, address, port)` asks a game server about itself. The only protocol is
`samp`, for SA-MP and open.mp servers. `port` goes from 1 to 65535.

```json
"permissions": { "query": ["samp"] }
```

**Returns** `{ password, players, max_players, hostname, gamemode, language }`, or `null` when
the server does not answer.
