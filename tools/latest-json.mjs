// Writes the `latest.json` that the in-app updater reads.
//
//   node tools/latest-json.mjs --version 0.2.0 --repo owner/name --installer <file> \
//        --signature <file.sig> [--notes <text>] [--out latest.json]
//
// Run by the release workflow after `cargo tauri build` has produced the
// installer and its `.sig`. The updater verifies the signature against the
// public key embedded in the application, so this file carries no trust of its
// own: it only says where the installer is and what it was signed as.
import { readFileSync, writeFileSync } from "node:fs";
import { basename } from "node:path";
import { pathToFileURL } from "node:url";

const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-[0-9A-Za-z.-]+)?$/;
const REPO = /^(?!\.+\/)[A-Za-z0-9._-]+\/(?!\.+$)[A-Za-z0-9._-]+$/;
// GitHub renames an uploaded file whose name has other characters (spaces
// become dots), and the address written here would then lead nowhere.
const ASSET = /^[A-Za-z0-9._-]+$/;

/**
 * The content of `latest.json`.
 *
 * @param {object} options `version`, `repo` (`owner/name`), `installerName`,
 *   `signature` (the text of the `.sig` file), `notes` and `date` (a Date).
 * @returns {object}
 */
export function latest({ version, repo, installerName, signature, notes = "", date = new Date() }) {
  if (!SEMVER.test(version)) throw new Error(`version ${version} is not semver (1.2.3)`);
  if (!REPO.test(repo)) throw new Error("repo must look like owner/name");
  if (!installerName || !ASSET.test(installerName))
    throw new Error("the installer name must be a file name of letters, digits, dots, hyphens and underscores");
  if (!signature || signature.trim() === "") throw new Error("the signature file is empty");
  return {
    version,
    notes,
    pub_date: date.toISOString(),
    platforms: {
      "windows-x86_64": {
        signature: signature.trim(),
        url: `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(installerName)}`,
      },
    },
  };
}

function option(args, name) {
  const at = args.indexOf(`--${name}`);
  return at >= 0 ? args[at + 1] : undefined;
}

function main(args) {
  const installer = option(args, "installer");
  const signature = option(args, "signature");
  if (!installer || !signature) throw new Error("give --installer and --signature");
  const document = latest({
    version: option(args, "version"),
    repo: option(args, "repo"),
    installerName: basename(installer),
    signature: readFileSync(signature, "utf8"),
    notes: option(args, "notes") ?? "",
  });
  writeFileSync(option(args, "out") ?? "latest.json", `${JSON.stringify(document, null, 2)}\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}
