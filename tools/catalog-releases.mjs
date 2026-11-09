// Keeps the catalog bundles on GitHub releases and counts installs from them.
//
//   node tools/catalog-releases.mjs upload --channel <c> --repo <owner/name> --dir <bundles>
//   node tools/catalog-releases.mjs prune  --channel <c> --repo <owner/name> --dir <bundles>
//                                          --ledger retired.json --plan plan.json
//   node tools/catalog-releases.mjs delete --repo <owner/name> --plan plan.json
//   node tools/catalog-releases.mjs stats  --repo <owner/name> [--ledger retired.json] [--out stats.json]
//
// Bundles are spread over 16 releases per channel by the folder name, because
// GitHub holds at most 1000 files per release; the application computes the
// same release (Channel::release in crates/rpp-catalog/src/index.rs). Each
// presence keeps its current bundle and the one before; older ones are
// deleted. GitHub forgets a deleted file's download count, so `prune` first
// writes that count to a ledger kept on the catalog-stats branch, and `stats`
// adds the ledger to the live counts. The token comes from GITHUB_TOKEN.
import { appendFileSync, existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { BUNDLE_NAME } from "./catalog.mjs";

/** Bundles kept per presence: the current one and the one before. */
export const KEEP = 2;
const CHANNELS = ["stable", "experimental"];
/** A presence folder name, as tools/catalog.mjs allows it. */
const FOLDER = /^[a-z0-9][a-z0-9-]{0,63}$/;

/** Release tag that holds a channel's bundles; the application asks for the same one. */
export function releaseFor(channel) {
  return channel === "experimental" ? "catalog-files-experimental" : "catalog-files";
}

/** Every release tag of a channel. The 16 of version 0.2.0 were deleted; their counts are in the ledger. */
export function releasesOf(channel) {
  return [releaseFor(channel)];
}

/** The folder a bundle file belongs to, or null for any other file. */
function folderOf(name) {
  return BUNDLE_NAME.exec(name ?? "")?.[1] ?? null;
}

/**
 * Release files to delete: per presence, everything but the bundles the new
 * index names and, besides those, the newest others up to `keep` in all.
 * Files that are not bundles are never touched.
 *
 * @param {{id: number, name: string, created_at: string}[]} assets Files of one release.
 * @param {Set<string>} current Bundle names the published index uses.
 * @param {number} keep Bundles kept per presence.
 * @returns {object[]} The assets to delete.
 */
export function prunePlan(assets, current, keep = KEEP) {
  const byFolder = new Map();
  for (const asset of assets) {
    const folder = folderOf(asset?.name);
    if (folder) byFolder.set(folder, [...(byFolder.get(folder) ?? []), asset]);
  }
  const doomed = [];
  for (const files of byFolder.values()) {
    const newestFirst = [...files].sort(
      (a, b) => String(b.created_at).localeCompare(String(a.created_at)) || b.id - a.id,
    );
    let kept = newestFirst.filter((asset) => current.has(asset.name)).length;
    for (const asset of newestFirst) {
      if (current.has(asset.name)) continue;
      if (kept < keep) kept += 1;
      else doomed.push(asset);
    }
  }
  return doomed;
}

/**
 * Installs per presence: live bundle downloads plus the ledger of deleted
 * bundles. A ledger entry whose file is still live is skipped, so a delete
 * that failed after the ledger was written is not counted twice.
 *
 * @param {{id: number, name: string, download_count: number}[]} live Files of every release of a channel.
 * @param {{folder: string, count: number}[]} retired Ledger entries of that channel, with `id`.
 * @returns {{folder: string, installs: number}[]} Most installed first.
 */
export function tally(live, retired = []) {
  const totals = new Map();
  const add = (folder, count) => {
    const valid = Number.isSafeInteger(count) && count > 0 ? count : 0;
    totals.set(folder, (totals.get(folder) ?? 0) + valid);
  };
  const liveIds = new Set();
  for (const asset of live) {
    const folder = folderOf(asset?.name);
    if (!folder) continue;
    liveIds.add(String(asset.id));
    add(folder, asset.download_count);
  }
  for (const entry of retired) {
    if (!liveIds.has(String(entry.id)) && FOLDER.test(entry.folder ?? "")) add(entry.folder, entry.count);
  }
  return [...totals]
    .map(([folder, installs]) => ({ folder, installs }))
    .sort((a, b) => b.installs - a.installs || a.folder.localeCompare(b.folder));
}

/** The ledger of deleted bundles, empty when the file does not exist yet. */
export function readLedger(path) {
  if (!path || !existsSync(path)) return { assets: {} };
  const ledger = JSON.parse(readFileSync(path, "utf8"));
  return { assets: ledger && typeof ledger.assets === "object" ? ledger.assets : {} };
}

/** A small client for the parts of the GitHub API used here. */
function github(repo) {
  if (!/^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/.test(repo)) throw new Error("give --repo <owner/name>");
  const token = process.env.GITHUB_TOKEN;
  const headers = { accept: "application/vnd.github+json", "x-github-api-version": "2022-11-28" };
  if (token) headers.authorization = `Bearer ${token}`;
  const api = `https://api.github.com/repos/${repo}`;
  const ask = async (url, init = {}) => {
    const answer = await fetch(url, { ...init, headers: { ...headers, ...init.headers } });
    if (!answer.ok && answer.status !== 404)
      throw new Error(`${init.method ?? "GET"} ${url}: GitHub answered ${answer.status}`);
    return answer;
  };
  return {
    /** The release of a tag, or null. */
    async release(tag) {
      const answer = await ask(`${api}/releases/tags/${tag}`);
      return answer.status === 404 ? null : answer.json();
    },
    /** Creates a release that never becomes "latest": the updater reads that one. */
    async create(tag) {
      const body = JSON.stringify({
        tag_name: tag,
        name: `Catalog files (${tag})`,
        body: "Presence bundles the application installs. Their download counts are the catalog's install statistics.",
        make_latest: "false",
      });
      return (await ask(`${api}/releases`, { method: "POST", body })).json();
    },
    async assets(id) {
      const all = [];
      for (let page = 1; page <= 20; page++) {
        const batch = await (await ask(`${api}/releases/${id}/assets?per_page=100&page=${page}`)).json();
        all.push(...batch);
        if (batch.length < 100) break;
      }
      return all;
    },
    async upload(id, name, text) {
      const url = `https://uploads.github.com/repos/${repo}/releases/${id}/assets?name=${encodeURIComponent(name)}`;
      await ask(url, { method: "POST", body: text, headers: { "content-type": "application/json" } });
    },
    async remove(assetId) {
      await ask(`${api}/releases/assets/${assetId}`, { method: "DELETE" });
    },
  };
}

/** Bundle files in a folder, grouped by the release they belong to. */
function bundlesByRelease(dir, channel) {
  const groups = new Map();
  for (const name of readdirSync(dir).sort()) {
    const folder = folderOf(name);
    if (!folder) continue;
    const tag = releaseFor(channel);
    groups.set(tag, [...(groups.get(tag) ?? []), name]);
  }
  return groups;
}

function option(args, name, fallback) {
  const at = args.indexOf(`--${name}`);
  return at >= 0 ? args[at + 1] : fallback;
}

function channelOf(args) {
  const channel = option(args, "channel", "stable");
  if (!CHANNELS.includes(channel)) throw new Error("the channel is stable or experimental");
  return channel;
}

async function main(argv) {
  const [command, ...args] = argv;
  const gh = github(option(args, "repo", ""));

  if (command === "upload") {
    const channel = channelOf(args);
    const dir = option(args, "dir");
    for (const [tag, names] of bundlesByRelease(dir, channel)) {
      const release = (await gh.release(tag)) ?? (await gh.create(tag));
      const existing = new Set((await gh.assets(release.id)).map((asset) => asset.name));
      // A file already there is never replaced: that would reset its count.
      for (const name of names.filter((name) => !existing.has(name))) {
        await gh.upload(release.id, name, readFileSync(join(dir, name)));
        console.log(`${tag}: uploaded ${name}`);
      }
    }
    return;
  }

  if (command === "prune") {
    const channel = channelOf(args);
    const current = new Set(readdirSync(option(args, "dir")).filter((name) => folderOf(name)));
    const ledgerPath = option(args, "ledger");
    const ledger = readLedger(ledgerPath);
    const plan = [];
    for (const tag of releasesOf(channel)) {
      const release = await gh.release(tag);
      if (!release) continue;
      for (const asset of prunePlan(await gh.assets(release.id), current)) {
        ledger.assets[String(asset.id)] = {
          channel,
          folder: folderOf(asset.name),
          name: asset.name,
          count: asset.download_count,
        };
        plan.push({ id: asset.id, name: asset.name });
      }
    }
    // The ledger is written (and pushed by the workflow) before anything is deleted.
    writeFileSync(ledgerPath, `${JSON.stringify(ledger, null, 2)}\n`);
    writeFileSync(option(args, "plan", "plan.json"), `${JSON.stringify(plan, null, 2)}\n`);
    console.log(`${plan.length} old bundles to delete`);
    return;
  }

  if (command === "delete") {
    const plan = JSON.parse(readFileSync(option(args, "plan", "plan.json"), "utf8"));
    for (const { id, name } of plan) {
      await gh.remove(id);
      console.log(`deleted ${name}`);
    }
    return;
  }

  if (command === "stats") {
    const ledger = Object.entries(readLedger(option(args, "ledger")).assets).map(([id, entry]) => ({ id, ...entry }));
    const stats = { generated_at: Math.floor(Date.now() / 1000) };
    for (const channel of CHANNELS) {
      const live = [];
      for (const tag of releasesOf(channel)) {
        const release = await gh.release(tag);
        if (release) live.push(...(await gh.assets(release.id)));
      }
      stats[channel] = tally(
        live,
        ledger.filter((entry) => entry.channel === channel),
      );
    }
    writeFileSync(option(args, "out", "stats.json"), `${JSON.stringify(stats, null, 2)}\n`);
    const table = [
      "| Presence | Installs |",
      "| --- | ---: |",
      ...stats.stable.map((row) => `| ${row.folder} | ${row.installs} |`),
    ].join("\n");
    console.log(table);
    // In a workflow, the table also goes to the run's summary page.
    if (process.env.GITHUB_STEP_SUMMARY)
      appendFileSync(process.env.GITHUB_STEP_SUMMARY, `## Catalog installs (stable)\n\n${table}\n`);
    return;
  }

  throw new Error("commands: upload, prune, delete, stats");
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
