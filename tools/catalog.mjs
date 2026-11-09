// Builds and signs the catalog index of approved presences.
//
//   node tools/catalog.mjs keygen <private-key-file>
//   node tools/catalog.mjs build --commit <sha> [--channel stable|experimental]
//                                [--presences presences] [--out catalog.json]
//                                [--key-file <file> | RPP_CATALOG_KEY in the environment]
//   node tools/catalog.mjs verify <catalog.json> <public-key-base64>
//   node tools/catalog.mjs bundles [--channel stable|experimental] [--presences presences] --out <folder>
//
// `bundles` writes one file per presence version with all its files;
// `build --bundles` names those files in the index so the application
// downloads them from GitHub releases. Uploading them and counting their
// downloads is tools/catalog-releases.mjs. The application sends nothing for
// this: a download is the only signal.
//
// `build` lists every file of every folder under presences/ with its SHA-256
// and signs the result with an Ed25519 key. The application trusts the index
// through that signature and every file through the index, so the private key
// belongs in a protected CI secret, never in the repository.
import { createHash, createPrivateKey, createPublicKey, generateKeyPairSync, sign, verify } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCHEMA_VERSION = 1;
// Limits the application enforces when it reads the index.
const MAX_FILES = 128;
const MAX_FILE = 1024 * 1024;
const MAX_TOTAL = 8 * 1024 * 1024;
const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;
const FOLDER = /^[a-z0-9][a-z0-9-]{0,63}$/;
const SEGMENT = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;
const LANGUAGE = /^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$/;
const MAX_DESCRIPTION = 300;

// The closed list of tags and the newest format, from the file the application and the editor read.
const SCHEMA = new URL("../crates/rpp-app/templates/manifest.schema.json", import.meta.url);
const SCHEMA_PROPERTIES = JSON.parse(readFileSync(SCHEMA, "utf8")).properties;
const TAGS = new Set(SCHEMA_PROPERTIES.tags.items.enum);
const FORMAT = SCHEMA_PROPERTIES.format.maximum;

// The first 12 bytes of an Ed25519 public key in DER, before the 32 raw bytes.
const SPKI_PREFIX = Buffer.from("302a300506032b6570032100", "hex");

/** Every regular file under a folder, as POSIX paths relative to it. */
function filesIn(folder) {
  const found = [];
  const walk = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      // Hidden files (an editor's leftovers, the install marker) are not part of a presence.
      if (entry.name.startsWith(".") || entry.name === "jsconfig.json") continue;
      const path = join(directory, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.isFile()) found.push(relative(folder, path).split("\\").join("/"));
      else throw new Error(`${path}: only regular files and folders are allowed`);
    }
  };
  walk(folder);
  return found;
}

/**
 * The `description` of each language file of a folder, by language tag.
 *
 * The application shows the one in the language in use, as it does for an
 * installed presence.
 *
 * @param {string} where The presence folder.
 * @returns {Record<string, string>}
 */
function descriptionsOf(where) {
  const folder = join(where, "locales");
  const found = {};
  if (!existsSync(folder)) return found;
  for (const name of readdirSync(folder).sort()) {
    const tag = name.endsWith(".json") ? name.slice(0, -5) : null;
    if (tag === null || !LANGUAGE.test(tag)) continue;
    let table;
    try {
      table = JSON.parse(readFileSync(join(folder, name), "utf8"));
    } catch (error) {
      throw new Error(`${where}/locales/${name}: ${error.message}`);
    }
    if (typeof table?.description === "string" && table.description.trim() !== "") found[tag] = table.description;
  }
  return found;
}

/**
 * The index entry of one presence folder.
 *
 * @param {string} root Folder that holds the presences.
 * @param {string} folder
 * @returns {object}
 */
export function describe(root, folder) {
  const where = join(root, folder);
  if (!FOLDER.test(folder)) throw new Error(`${folder}: folder names use lower-case letters, digits and hyphens`);
  const manifest = JSON.parse(readFileSync(join(where, "manifest.json"), "utf8"));
  if (typeof manifest.name !== "string" || manifest.name.trim() === "")
    throw new Error(`${folder}: the manifest has no name`);
  if (!SEMVER.test(manifest.version)) throw new Error(`${folder}: version ${manifest.version} is not semver (1.2.3)`);
  const format = manifest.format ?? 1;
  if (!Number.isInteger(format) || format < 1 || format > FORMAT)
    throw new Error(`${folder}: format ${JSON.stringify(manifest.format)} is not a whole number from 1 to ${FORMAT}`);
  const unknown = (Array.isArray(manifest.tags) ? manifest.tags : []).filter((tag) => !TAGS.has(tag));
  if (unknown.length > 0) throw new Error(`${folder}: tags not on the list: ${unknown.join(", ")}`);
  const paths = filesIn(where);
  if (paths.length === 0 || paths.length > MAX_FILES) throw new Error(`${folder}: expected 1 to ${MAX_FILES} files`);
  const files = {};
  let total = 0;
  for (const path of paths) {
    if (!path.split("/").every((segment) => SEGMENT.test(segment)))
      throw new Error(`${folder}/${path}: unusual file name`);
    const bytes = readFileSync(join(where, path));
    if (bytes.length > MAX_FILE) throw new Error(`${folder}/${path}: larger than ${MAX_FILE} bytes`);
    total += bytes.length;
    files[path] = { sha256: createHash("sha256").update(bytes).digest("hex"), size: bytes.length };
  }
  if (total > MAX_TOTAL) throw new Error(`${folder}: larger than ${MAX_TOTAL} bytes`);
  if (!files[manifest.entry]) throw new Error(`${folder}: the script ${manifest.entry} is not in the folder`);
  const { en, ...descriptions } = descriptionsOf(where);
  for (const text of [manifest.description, en, ...Object.values(descriptions)])
    if (typeof text === "string" && [...text].length > MAX_DESCRIPTION)
      throw new Error(`${folder}: a description is longer than ${MAX_DESCRIPTION} characters`);
  return {
    name: manifest.name,
    folder,
    version: manifest.version,
    // The English language file wins over the manifest, as in the application.
    description: en ?? (typeof manifest.description === "string" ? manifest.description : null),
    descriptions,
    other_names: Array.isArray(manifest.other_names)
      ? manifest.other_names.filter((name) => typeof name === "string")
      : [],
    kind: typeof manifest.kind === "string" ? manifest.kind : null,
    author: typeof manifest.author === "string" ? manifest.author : null,
    color: typeof manifest.color === "string" ? manifest.color : null,
    tags: Array.isArray(manifest.tags) ? manifest.tags : [],
    // A logo kept as a file is already among the presence's files; only an address is listed.
    icon: typeof manifest.icon === "string" && manifest.icon.startsWith("https://") ? manifest.icon : null,
    // The application refuses an entry it is too old for before downloading it.
    format,
    files,
  };
}

/** Every presence a channel lists, described, folders sorted. */
function listed(root, channel) {
  if (channel !== "stable" && channel !== "experimental") throw new Error("the channel is stable or experimental");
  const presences = [];
  for (const entry of readdirSync(root, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    if (!entry.isDirectory() || entry.name.startsWith(".")) continue;
    const described = describe(root, entry.name);
    // The stable channel never lists a pre-release.
    if (channel === "stable" && described.version.includes("-")) continue;
    presences.push(described);
  }
  return presences;
}

export const BUNDLE_FORMAT = "rich-presence-plus/bundle";
/** A bundle's release file name: folder, version and 12 digits of its hash. */
export const BUNDLE_NAME = /^([a-z0-9][a-z0-9-]{0,63})_([0-9A-Za-z.-]+)_([0-9a-f]{12})\.json$/;

/**
 * The bundle of one described presence: every file in base64, paths sorted.
 *
 * The name carries a hash of the text, so a version published again with
 * other files gets a new release file instead of replacing the counted one.
 *
 * @param {string} root Folder that holds the presences.
 * @param {object} entry An entry from `describe`.
 * @returns {{name: string, text: string}} File name and contents.
 */
export function bundleOf(root, entry) {
  // GitHub renames "+" in release file names, which would break the link.
  if (entry.version.includes("+"))
    throw new Error(`${entry.folder}: build metadata (+) cannot be in a catalog version`);
  const files = {};
  for (const path of Object.keys(entry.files).sort()) {
    files[path] = readFileSync(join(root, entry.folder, path)).toString("base64");
  }
  const text = JSON.stringify({ format: BUNDLE_FORMAT, folder: entry.folder, version: entry.version, files });
  const hash = createHash("sha256").update(text).digest("hex").slice(0, 12);
  return { name: `${entry.folder}_${entry.version}_${hash}.json`, text };
}

/**
 * Builds the payload text that gets signed.
 *
 * @param {object} options `root`, `commit`, `channel`, `generatedAt` (Unix
 *   seconds) and `bundles` (name each presence's release bundle).
 * @returns {string} The payload as JSON text.
 */
export function buildPayload({ root, commit, channel = "stable", generatedAt, bundles = false }) {
  if (!/^[0-9a-f]{40}$/.test(commit)) throw new Error("expected the full 40-character commit SHA");
  const presences = listed(root, channel).map((entry) =>
    bundles ? { ...entry, bundle: bundleOf(root, entry).name } : entry,
  );
  return JSON.stringify({
    schema_version: SCHEMA_VERSION,
    channel,
    commit,
    generated_at: generatedAt ?? Math.floor(Date.now() / 1000),
    presences,
  });
}

/**
 * Signs the text from `buildPayload` into the text of `catalog.json`.
 *
 * @param {string} payload
 * @param {string} pem The private key, PKCS#8 PEM.
 * @param {string} keyId Identifier the application knows the public key by.
 * @returns {string}
 */
export function envelope(payload, pem, keyId) {
  const signature = sign(null, Buffer.from(payload, "utf8"), createPrivateKey(pem)).toString("base64");
  return `${JSON.stringify({ key_id: keyId, payload, signature }, null, 2)}\n`;
}

/** The 32 raw bytes of a public key, as base64: the form the application embeds. */
export function publicKeyOf(pem) {
  const der = createPublicKey(createPrivateKey(pem)).export({ type: "spki", format: "der" });
  return der.subarray(SPKI_PREFIX.length).toString("base64");
}

/** Checks a `catalog.json` text against a base64 public key. */
export function check(text, publicKey) {
  const { payload, signature } = JSON.parse(text);
  const key = createPublicKey({
    key: Buffer.concat([SPKI_PREFIX, Buffer.from(publicKey, "base64")]),
    format: "der",
    type: "spki",
  });
  return verify(null, Buffer.from(payload, "utf8"), key, Buffer.from(signature, "base64"));
}

function option(args, name, fallback) {
  const at = args.indexOf(`--${name}`);
  return at >= 0 ? args[at + 1] : fallback;
}

async function main(argv) {
  const [command, ...args] = argv;
  if (command === "keygen") {
    const target = args[0];
    if (!target) throw new Error("usage: keygen <private-key-file>");
    if (existsSync(target)) throw new Error(`${target} already exists; refusing to overwrite a key`);
    const { privateKey } = generateKeyPairSync("ed25519");
    const pem = privateKey.export({ type: "pkcs8", format: "pem" });
    writeFileSync(target, pem, { mode: 0o600 });
    console.log(`Private key written to ${target}. Keep it out of the repository.`);
    console.log(`Public key (put it in crates/rpp-catalog/src/keys.rs): ${publicKeyOf(pem)}`);
    return;
  }
  if (command === "build") {
    const root = option(args, "presences", join(dirname(fileURLToPath(import.meta.url)), "..", "presences"));
    const keyFile = option(args, "key-file");
    const pem = keyFile ? readFileSync(keyFile, "utf8") : process.env.RPP_CATALOG_KEY;
    if (!pem) throw new Error("give --key-file or set RPP_CATALOG_KEY");
    const payload = buildPayload({
      root,
      commit: option(args, "commit", ""),
      channel: option(args, "channel", "stable"),
      bundles: args.includes("--bundles"),
    });
    const text = envelope(payload, pem, option(args, "key-id", "main"));
    const out = option(args, "out", "catalog.json");
    writeFileSync(out, text);
    console.log(`${out}: ${JSON.parse(payload).presences.length} presences`);
    return;
  }
  if (command === "verify") {
    const [file, key] = args;
    if (!file || !key) throw new Error("usage: verify <catalog.json> <public-key-base64>");
    if (!check(readFileSync(file, "utf8"), key)) throw new Error("the signature does not match");
    console.log("The signature matches.");
    return;
  }
  if (command === "bundles") {
    const root = option(args, "presences", join(dirname(fileURLToPath(import.meta.url)), "..", "presences"));
    const out = option(args, "out");
    if (!out) throw new Error("usage: bundles --out <folder>");
    mkdirSync(out, { recursive: true });
    const written = listed(root, option(args, "channel", "stable")).map((entry) => bundleOf(root, entry));
    for (const { name, text } of written) writeFileSync(join(out, name), text);
    console.log(`${out}: ${written.length} bundles`);
    return;
  }
  throw new Error("commands: keygen, build, verify, bundles");
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
