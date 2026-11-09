# The catalog

Approved presences live in this repository, under `presences/`. The
application lists them under Presences, Add more, and installs them from GitHub.

## How trust works

1. A maintainer runs the `catalog` workflow. After a maintainer approves the
   `catalog-publication` environment, it builds an index of every file of every
   presence (path, size, SHA-256) at one commit and signs it with an Ed25519 key.
   The signed `catalog.json` goes to the `catalog` branch (`catalog-experimental`
   for pre-releases). The branch holds one commit, the newest index; each
   publish replaces it.
2. The application downloads that file and checks the signature against the
   public key built into it. It refuses an index that is unsigned, signed by
   another key, for another channel, or older than one it accepted before. It
   keeps the verified copy for five minutes, then asks again with the tag
   GitHub gave it; an index that did not change is not downloaded again.
3. Each presence version is also uploaded as one bundle file to a GitHub
   release, and the index names it. GitHub holds at most 1000 files per
   release, so bundles are spread over 16 releases per channel
   (`catalog-files-0` to `-f`, `catalog-files-experimental-0` to `-f`) by the
   first hex digit of the SHA-256 of the folder name; the application works
   out the same release. Each presence keeps its current bundle and the one
   before. The application downloads the bundle and checks every file
   in it against the size and SHA-256 the index gives; a bundle with any other
   file, a missing one or a changed one is refused. When the index names no
   bundle, or the bundle cannot be used, each file is downloaded from the exact
   commit in the index and checked the same way. The manifest has to match the
   index, and the script has to be one of the signed files.
4. The presence is staged in a hidden folder and swapped in with a rename. If
   any step fails, the installed copy is left as it was.

A merge into `main` publishes nothing: only a signed index makes a presence
installable.

## Install statistics

GitHub counts the downloads of each release file. The `catalog-stats`
workflow runs once a day, adds the counts of every bundle of a presence (all
versions, installs and updates together) and commits the result as
`stats.json` to the `catalog-stats` branch; the run's summary shows the table.
To read it by hand:

```
node tools/catalog-releases.mjs stats --repo manucabral/RichPresencePlus
```

GitHub forgets a file's count when the file is deleted. Before the publish
workflow deletes old bundles, it writes their counts to `retired.json` on the
same branch and pushes it; the totals add that ledger to the live counts. If a
delete fails after that, the file is still live and its ledger entry is
skipped, so nothing is counted twice.

The application sends nothing for this; the download it already makes is the
only signal, and GitHub does not say who made it. A bundle file is never
replaced, because that would reset its count: its name carries a hash of its
contents (`<folder>_<version>_<hash>.json`), so different contents get a new
file. Versions with build metadata (`+…`) cannot be published, since GitHub
renames that character in file names.

## Adding a presence

The Publish tab of the Developer page checks these rules, writes a clean copy of the folder and
opens the upload page of GitHub. By hand, open a pull request that adds `presences/<folder>/` with:

- `manifest.json` and the script it names in `entry`.
- `locales/en.json`, and a translation per language you can write (see
  [TRANSLATING.md](TRANSLATING.md)).

Rules the check enforces, the same ones the application enforces:

- The folder name uses lower-case letters, digits and hyphens.
- The version is semver (`1.2.0`). Pre-releases (`1.2.0-rc.1`) only go to the
  experimental channel.
- At most 128 files, 1 MiB each and 8 MiB in total. File names use letters,
  digits, dot, underscore and hyphen; hidden files are left out.
- Tags come from the list in `manifest.schema.json`. A word off the list stops the check.
- The manifest asks for only the permissions the script uses.

The `catalog` workflow runs these checks on every pull request with a
throwaway key. Presences are code that runs on users' computers inside the
script sandbox: reviewers read the script and the permissions before approving.

## Presences the catalog installs and the ones you make

A presence installed from the catalog gets a hidden `.catalog.json` marker in
its folder. The marker lists every file with its SHA-256 and carries a seal: an
HMAC made with a secret that the application creates on first run
(`catalog.key` in its data folder) and that never leaves the computer. Only
marked folders can be updated or removed from the Add more tab of Presences.
Folders you create or copy by hand are never touched.

The application ships no presences: people install the ones they want from
Presences, Add more. A presence starts on when someone installs, creates or
copies it; one that arrives any other way (an update of the application, a
folder dropped in by hand) waits until it is turned on, and an update of an
installed presence keeps it on or off as it was. Development builds also read
the repository's `presences/` folder, as bundled presences, to work on them.

A catalog presence cannot be modified. The application never edits or watches
one, and the Presences page marks each presence as bundled, catalog or yours;
only yours can be edited. To change one, copy it from the Developer page: the
copy is yours, and the original author moves to `contributors`.

If a file of an installed presence is edited, added or deleted, or the marker
is copied from elsewhere, the seal no longer matches. The folder is then not
loaded: the Add more tab shows "Files changed after installing" with a
Reinstall button, and the Dashboard lists the folder under the presences that
did not load. Reinstalling restores the reviewed files. A modified copy is not
kept in the history, and a kept copy that was changed there is not restored.

## Names

A name is the same as another when it only differs in case or separators:
`My Site`, `my_site` and `MY-SITE` are one presence. Names also cannot be
Windows device names (`con`, `nul`, `com1`, `lpt1`, ...) as folders or files.
When two folders claim one name, the order is: bundled and catalog presences
first (an installed copy replaces the bundled one of that name), then
the presences you wrote. A folder you wrote can never take the name of a
bundled or catalog presence; the Dashboard lists it with the name that won.
Creating or copying a presence under a name that exists is refused, and so is
installing a catalog presence while one of yours has its name.

## Previous versions

When an update replaces an installed presence, the old copy is kept in
`presence-history/<folder>/<version>` next to the settings, and only the two
newest are kept. The page of each presence in Add more lists them under "Previous versions" with a
Restore button; restoring swaps the current copy into the history, so it can be
restored back. A failed update leaves the history as it was.
