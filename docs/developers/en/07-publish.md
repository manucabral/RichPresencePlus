---
title: Publish your presence
description: Propose your presence to the Rich Presence Plus catalog with a pull request, so everyone with the app can install it.
---

# Publish your presence

A presence you write runs on your computer. To let everyone install it, propose it to the
catalog. People read it first, and once it is approved anyone with the application can
download it from the Catalog page.

## Check that it is ready

Open the Publish tab of the Developer page and pick your presence. It checks these rules, the
same ones GitHub checks:

- The folder name uses lower-case letters, digits and hyphens.
- The version looks like `1.2.0`.
- It has a description, an author and at least one tag from the list.
- It has `locales/en.json`.
- It has at most 128 files, 1 MB each and 8 MB in total.
- No file holds a password or a key.

Fix what is marked and press "Check again".

## Send it

You need a GitHub account.

1. Press "Prepare folder". The application makes a clean copy, without the files that only
   your editor uses, and shows it in File Explorer.
2. Press "Open GitHub". The page asks you to make your own copy of the project the first time.
3. Drag the folder to the page.
4. Describe your presence and create the pull request.

GitHub checks the rules again, by itself, within a few minutes.

## What the review looks at

Someone reads your script and your permissions before approving. A presence is refused if it:

- Asks for a site, a program or a permission it does not use.
- Sends what it reads anywhere other than Discord.
- Shows private data, such as messages, account names or addresses.
- Copies the name or the look of something it is not.

The review may ask for changes in the pull request. When it is approved, the presence reaches
the catalog with the next publication.

## Update your presence

Change your folder, raise `version` and repeat the steps. People who installed it get the new
version from the Catalog page. If it asks for more access than before, they have to accept
that first, and the old version keeps running until they do.
