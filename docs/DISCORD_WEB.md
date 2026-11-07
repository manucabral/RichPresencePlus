# Discord in the browser

Research for roadmap item 5.1, read on 2026-11-05. Rich Presence over the local pipe
needs the Discord desktop app. This document asks whether the official Social SDK can
reach someone who uses Discord only in a browser.

## Decision

Wait. Nothing gets built now. The app stays as it is: it needs the desktop client, and
the Dashboard already says so when that client is closed.

The reasons, in order:

1. The SDK is made for games. Discord's docs present it as the way to add Rich Presence,
   friends and voice to a game, and full access asks for game features such as joining
   through Discord. Rich Presence Plus is not a game and shows other programs' activity.
2. The Social SDK Terms could not be read: the page answers 403 to automated requests.
   Someone has to read them in a browser before any design. Without that, using the SDK
   here could be a breach, and the terms of a platform the whole app depends on are not
   worth guessing at.
3. It adds a Discord login to an app that has no accounts today, and tokens to store.
   The privacy policy would have to change, and "no accounts" is part of what the app
   promises.
4. It adds a native C++ library to a program that weighs 12 MB in memory.

## What the documentation says

| Question | Answer | Source |
| --- | --- | --- |
| Which platforms? | Windows, macOS, Android, iOS, Xbox and PlayStation. Linux is experimental. No browser build. | Platform Compatibility |
| Which languages? | Standalone C++, Unity and Unreal. | Platform Compatibility |
| Does it need the desktop app? | Without a login it talks to the running client over RPC, which is what we already do. With a linked account the guide does not state how the presence travels. | Setting Rich Presence |
| What must the person grant? | An OAuth2 link with the scopes `openid` and `sdk.social_layer_presence`. Access tokens last about 7 days, with refresh tokens. | Account Linking |
| One application or several? | The presence shows the registered application's name, and `SetName` can override it. It says nothing about one application carrying many presences. | Setting Rich Presence |
| Limits? | Text and voice have minimal limits until an application review. The numbers for Rich Presence were not found. | Communication Features |
| Terms? | Not read: see above. | Social SDK Terms |

A second way exists on paper: the OAuth2 scope `activities.write`, which community
references describe as updating a user's presence without a client. It needs Discord's
approval and the official docs found do not describe it, so it is not a plan.

The ways ruled out by the roadmap stay ruled out: injecting code into discord.com, and
publishing with a user's token.

## What would change the decision

- The Social SDK Terms allow an application that is not a game.
- Discord documents that a linked account's presence shows with no desktop client.
- People ask for it. Nobody has yet, and the app is not published.

If those hold, the work starts behind the `Connector` and `Connection` boundary in
`rpp-discord`, which already keeps presences and the policy apart from how an activity
reaches Discord.

## Sources

- [Platform Compatibility](https://docs.discord.com/developers/discord-social-sdk/core-concepts/platform-compatibility)
- [Setting Rich Presence](https://docs.discord.com/developers/discord-social-sdk/development-guides/setting-rich-presence)
- [Account Linking](https://docs.discord.com/developers/platform/account-linking)
- [Communication Features](https://docs.discord.com/developers/discord-social-sdk/core-concepts/communication-features)
- [Social Layer for Games](https://docs.discord.com/developers/platform/social-layer)
- [Social SDK Terms](https://support-dev.discord.com/hc/en-us/articles/30225844245271-Discord-Social-SDK-Terms) (not readable by tools)
- [discord-api-types: OAuth2 scopes](https://discord-api-types.dev/api/discord-api-types-v10/enum/OAuth2Scopes)
