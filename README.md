<p align="center">
<img src="frontend/public/logo.svg" width="200" alt="Rich Presence Plus logo">
</p>

<h1 align="center">Rich Presence Plus</h1>

<div align="center">
    <a href="https://github.com/manucabral/RichPresencePlus/releases/latest"><img src="https://img.shields.io/github/v/release/manucabral/RichPresencePlus?label=version" alt="Latest version"></a>
    <a href="https://github.com/manucabral/RichPresencePlus/releases"><img src="https://img.shields.io/github/downloads/manucabral/RichPresencePlus/total" alt="Downloads"></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/manucabral/RichPresencePlus" alt="License"></a>
    <a href="https://discord.gg/NjXFRqWarc"><img src="https://img.shields.io/discord/1118321148007022632?label=discord" alt="Discord"></a>
</div>

<p align="center">
Shows on your Discord profile what you watch, listen to and play: the video, the song, the game.<br>
Free and open source, for Windows.
</p>

<p align="center">
<a href="https://github.com/manucabral/RichPresencePlus/releases/latest"><b>Download</b></a> |
<a href="https://manucabral.github.io/RichPresencePlus/">Website</a> |
<a href="https://manucabral.github.io/RichPresencePlus/presences/">Presences</a> |
<a href="https://discord.gg/NjXFRqWarc">Discord</a>
</p>

![The Rich Presence Plus panel](website/public/shots/panel-en-1280.webp)

## Getting started

1. Download the installer from the [latest release](https://github.com/manucabral/RichPresencePlus/releases/latest) and run it. Windows may warn that the publisher is unknown while the installer is not signed yet: choose **More info** and then **Run anyway**.
2. Open Discord for desktop.
3. In the app, go to **Presences, Add more** and install the ones you want. Each starts on when you add it, and you can turn it off at any time.
4. For websites such as YouTube or Netflix, add the browser extension. It is in review in the Chrome, Edge and Firefox stores; the links will be here and on the app's **Browser** page once it is approved.

The app updates itself: when a new version comes out, the Panel offers it.

## What it does

- **Presences you choose.** The app comes empty. You install presences from a reviewed catalog, and nothing runs until you pick it.
- **Websites and programs.** Videos, music and series in the browser; games, editors and media players on the desktop. Web Media covers any site you allow.
- **Clear about what it reads.** Each presence lists what it reads before you install it, and an update that asks for more waits for your yes.
- **Privacy rules.** Hide titles, buttons, links or timers, leave out sites and programs, and hide everything while OBS, Zoom or another program runs, or at certain hours.
- **Your own statuses.** Write custom statuses, save sets of rules as profiles, and choose what wins when several things play.
- **Light.** About 7 MB of memory and under 0.1% of a core with the window closed.
- **English and Spanish.**

Everything stays on your computer: the app talks to Discord through Discord's own local connection, and the extension sends what it reads only to the app. The [privacy policy](https://manucabral.github.io/RichPresencePlus/privacy/) has the details.

## Browsers

The extension works in Chrome, Edge and Firefox, and finds the app by itself. Other Chromium browsers that install extensions from the Chrome Web Store can connect with the pairing code from the app's **Browser** page.

## Ask for a presence

Missing a site, game or program? Ask for it in the [Discord server](https://discord.gg/NjXFRqWarc) (#presence-requests) or open an [issue](https://github.com/manucabral/RichPresencePlus/issues/new/choose). The [project board](https://github.com/users/manucabral/projects/4) shows what is being worked on.

## Write a presence

Presences are small JavaScript files with a manifest that says what they read. The [developer guide](docs/developers/en/01-home.md) ([en español](docs/developers/es/01-home.md)) goes from the first file to publishing, and [docs/CATALOG.md](docs/CATALOG.md) explains how a presence is reviewed and signed into the catalog. The app's **Developer** page creates, tests and prepares a presence for you.

## Build from source

The app is written in Rust with Tauri, its window in Preact, and the extension in plain JavaScript. You need Rust (stable), Node.js with pnpm, and the Tauri CLI (`cargo install tauri-cli`).

```
pnpm --dir frontend install
cargo tauri dev
```

To build the installer:

```
cargo tauri build
```

Before sending a change, `cargo fmt --check`, `cargo clippy --all-targets` and `cargo test` must pass. [docs/DEVELOPING.md](docs/DEVELOPING.md) has the details and [CONTRIBUTING.md](CONTRIBUTING.md) explains how changes are reviewed.

## Community

- [Discord server](https://discord.gg/NjXFRqWarc): help, news and presence requests.
- [Discussions](https://github.com/manucabral/RichPresencePlus/discussions): questions and ideas.
- [Issues](https://github.com/manucabral/RichPresencePlus/issues/new/choose): bugs.

## Roadmap

- A signed installer.
- The browser extension in the Chrome, Edge and Firefox stores.
- More presences in the catalog.
- More languages, starting with Portuguese and Russian.

The first version, written in Python, is in the [`old-rpp`](https://github.com/manucabral/RichPresencePlus/tree/old-rpp) branch and the [v0.1.1-rc](https://github.com/manucabral/RichPresencePlus/releases/tag/v0.1.1-rc) release.

## Contributors

<a href="https://github.com/manucabral/RichPresencePlus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=manucabral/RichPresencePlus" alt="Contributors">
</a>

Made with [contrib.rocks](https://contrib.rocks). Licensed under the [MIT License](LICENSE).
