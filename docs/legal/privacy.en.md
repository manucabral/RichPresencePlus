# Privacy policy

Last updated: 2026-11-04. Applies to the Rich Presence Plus desktop application, its browser extension, its catalog and this website.

## In short

Rich Presence Plus shows on your Discord profile what you play, watch or listen to. It has no accounts, no analytics and no servers of its own, and the project receives none of the data described here. It works on your computer. What leaves it is the activity you publish on Discord and the requests listed below.

## What the application reads

The application reads only what each presence you turn on declares in its manifest. You can see that list before turning a presence on.

| Source | What it reads | Who uses it |
| --- | --- | --- |
| Pages in your browser | The address, the title and the values the presence declares, such as a video title or the playback position. Some presences take one of those values from what the page loads from its site, such as the name of a song; the rest of what the page loads is not kept | Web presences |
| Running programs | Their names | Presences that declare it |
| Active window | Its program and title | Presences that declare it |
| Media session | The title, artist and state that Windows reports | Presences that declare it |
| Files | Files in the folders the manifest lists, such as a game's logs | Presences that declare it |
| Command lines | Only the options the manifest lists, for one program. Other options, such as passwords, are never returned | Presences that declare it |
| Discord | Your user name, from the local connection, to show which account is in use | The application |

Presences run in a sandbox and cannot read anything outside their declared list.

## What the browser extension reads

The extension works on sites you allow, one at a time, for presences you turn on. It reads nothing in private windows, and nothing while you pause it.

It sends what it reads only to the application on your computer, over `127.0.0.1`, on a connection that both sides authenticate and that is encrypted. The extension itself sends nothing to any server.

## What leaves your computer

- **Discord.** The application gives your activity to the Discord app on your computer, and Discord shows it on your profile to whoever your Discord settings allow. Your rules apply first: excluded sites and programs, hidden titles, buttons, links and timers, the "service only" mode, pause, and hiding while you are away. Discord handles this data under its own [privacy policy](https://discord.com/privacy). Pictures and links in an activity are addresses that Discord may load.
- **GitHub.** The application asks GitHub for new versions and for the presence catalog, and downloads what you install. GitHub sees your IP address, like any site you connect to, under its [privacy statement](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement). Download counts are public totals that do not say who downloaded. Your activity is not sent.
- **Hosts a presence declares.** A presence may ask the hosts in its manifest for public data, such as a game's name, or ask a game server for its public information. Those servers see your IP address and the request.
- **Pictures.** Presence logos are files of the application. The window can also show the picture of an activity, such as a video's thumbnail, which loads from the site that holds it. That site sees your IP address and the address of the picture.

Nothing is sold, and nothing is used for advertising or profiling. The application makes no automated decisions about you.

If you turn on Overlays and Stream Deck in Settings, programs on your computer can read what Discord shows through a local address. Other computers and websites cannot.

## What stays on your computer

| Data | Where | How long |
| --- | --- | --- |
| Settings, favorites, custom statuses and profiles | Application data folder | Until you delete them or uninstall |
| Pairing key | Application data folder and the browser's extension storage | Until you uninstall or remove the extension |
| Log files, without page addresses, titles or activities | Application data folder | Seven daily files |
| Installed presences | Application data folder | Until you remove them; two previous versions are kept |
| Developer versions, test cases and exports | Application data folder | Up to 30 versions per presence; exports until you delete them |
| Time each presence was on display, only if you turn it on in Your week: names and minutes | Application data folder | 56 days, or until you delete it |
| Browser sessions shown in the application | Memory | Until the application closes |
| Interface cache | Application data folder, by WebView2 | Until you uninstall |

The installer can also add entries to your user registry: starting with Windows, if you choose it, and the entries that let browsers find the application. Uninstalling removes them. It removes your data when you choose so in the uninstaller. Removing the extension removes its data from the browser.

## This website

The website uses no cookies, no analytics and no advertising, and it loads nothing from other servers. It is hosted on GitHub Pages, which keeps technical logs, including IP addresses, under the GitHub statement linked above. Links to other sites follow those sites' own policies.

## Your rights

The project holds no personal data about you, so there is nothing for it to access, correct or delete. You control everything stored on your computer and can delete it at any time. For data held by Discord or GitHub, use their tools.

If data protection law applies to you, you keep your rights under it, including the right to complain to your supervisory authority. In Argentina that is the Agencia de Acceso a la Información Pública ([AAIP](https://www.argentina.gob.ar/aaip/datospersonales)).

## Children

Rich Presence Plus is not directed at children. Using it requires a Discord account, so you must meet the minimum age Discord sets in your country.

## Chrome Web Store

The use of information received through the extension adheres to the [Chrome Web Store User Data Policy](https://developer.chrome.com/docs/webstore/program-policies/), including the Limited Use requirements. Data is used only to show your activity, and no person reads it.

## Changes and contact

Changes are published here with a new date, and important ones are listed in the release notes. For questions, open an issue at [github.com/manucabral/RichPresencePlus](https://github.com/manucabral/RichPresencePlus/issues).
