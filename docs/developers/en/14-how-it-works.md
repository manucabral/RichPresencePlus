---
title: How it works
description: What happens between a presence and Discord, and why a presence is built the way it is.
---

# How it works

## One round

Every `interval` seconds, for each presence that is on:

1. The application calls `read` with the time, the pages the extension reported and the options.
2. If `read` returned `null`, the presence has nothing to show this round.
3. It calls `activity` and `meta` with what `read` returned.
4. It checks the activity against Discord's limits and applies the person's privacy rules.
5. It compares every presence with something to show and sends one to Discord.

`read` gathers and `activity` presents. When a site changes its layout, you touch `read` and
the manifest. When you want a new button, you touch `activity`.

## Why the script does not run in the page

The extension reads only what the manifest names, with code of its own that never changes.
Your script receives the values in the application, far from the page. So a page can never run
your code, and your code can never see a password field, a message or a cookie that the
manifest did not name. Reviewers can read the manifest and know everything a presence reads.

## Why a script starts with nothing

The script runs in a small JavaScript engine inside the application, with no network, no files
and no `eval`. Each permission adds one door to `host`, and the application checks every call
against the manifest. People see the list before they turn a presence on, and an update that
asks for more waits for their answer.

## Which presence wins

Only one activity shows at a time. Each presence has a priority from 0 to 100: the person's
choice, or the default for its `kind` (`game` 80, `video` 60, `music` 40, `other` 20). A status
the person writes by hand always wins.

- A paused presence gives way to one that plays.
- `foreground` raises a presence while its window has focus.
- On a tie, the activity already on display stays, so the card does not jump.

## What privacy does

Before anything reaches Discord, the person's rules can exclude a site or a program, hide the
titles or the buttons and links of one presence, show only the name of the service, or hide
everything after some time away from the keyboard. Excluding a site or a program looks at
`domains` and at `application` in `meta`, which is why a presence declares them.

## Why the limits

Many presences can run at once on a computer that is also playing a game. The time and memory
limits keep one slow script from costing the others, and a script that goes over a limit only
loses that round.
