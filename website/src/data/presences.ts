/**
 * The presences of the repository, read when the site is built, so the site
 * lists what exists and what each one can read, never a list kept by hand.
 */
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { categoryOf } from "../../../frontend/src/shared/browse.ts";

/** One thing a presence is allowed to read. */
export type Read =
  | { kind: "page"; hosts: string[]; values: string[] }
  | { kind: "sites" }
  | { kind: "process" | "window" | "media" }
  | { kind: "files"; paths: string[] }
  | { kind: "http"; hosts: string[] }
  | { kind: "args"; programs: string[] }
  | { kind: "query"; protocols: string[] };

export interface Listed {
  name: string;
  folder: string;
  category: string;
  color: string;
  /** Description by language; English is always there. */
  description: Record<string, string>;
  /** Other names it is searched by. */
  otherNames: string[];
  reads: Read[];
}

interface Manifest {
  name: string;
  kind?: string;
  color?: string;
  description?: string;
  tags?: string[];
  other_names?: string[];
  page?: { matches?: string[]; extract?: Record<string, unknown>; frames?: { matches?: string[]; extract?: Record<string, unknown> } };
  permissions?: {
    http?: string[];
    process?: boolean;
    window?: boolean;
    media?: boolean;
    files?: string[];
    args?: Record<string, string[]>;
    query?: string[];
  };
}

/** The host of a match pattern such as `*://www.netflix.com/*`. */
export function hostOf(pattern: string): string | null {
  return /^[^:]+:\/\/([^/]+)\//.exec(pattern)?.[1] ?? null;
}

/** Everything a manifest declares, in a fixed order. */
export function readsOf(manifest: Manifest): Read[] {
  const reads: Read[] = [];
  const { page, permissions: allowed = {} } = manifest;
  if (page) {
    const hosts = [...(page.matches ?? []), ...(page.frames?.matches ?? [])].map(hostOf).filter((host): host is string => host !== null);
    if (hosts.includes("*")) reads.push({ kind: "sites" });
    else reads.push({ kind: "page", hosts: [...new Set(hosts)], values: Object.keys({ ...page.extract, ...page.frames?.extract }) });
  }
  if (allowed.process) reads.push({ kind: "process" });
  if (allowed.window) reads.push({ kind: "window" });
  if (allowed.media) reads.push({ kind: "media" });
  if (allowed.files?.length) reads.push({ kind: "files", paths: allowed.files });
  if (allowed.args) reads.push({ kind: "args", programs: Object.keys(allowed.args) });
  if (allowed.query?.length) reads.push({ kind: "query", protocols: allowed.query });
  if (allowed.http?.length) reads.push({ kind: "http", hosts: allowed.http });
  return reads;
}

/** Text color for a label on a `#rrggbb` color: near black on light, near white on dark. */
export function inkOn(hex: string): string {
  const channel = (at: number) => {
    const value = Number.parseInt(hex.slice(at, at + 2), 16) / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  const luminance = 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
  return luminance > 0.179 ? "#08090a" : "#f7f8f8";
}

function json<T>(path: string): T {
  return JSON.parse(readFileSync(path, "utf8")) as T;
}

/** Reads every folder under `presences`, sorted by name. */
export function loadPresences(root: string): Listed[] {
  const listed: Listed[] = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name, "manifest.json");
    if (!entry.isDirectory() || !existsSync(path)) continue;
    const manifest = json<Manifest>(path);
    const description: Record<string, string> = { en: manifest.description ?? "" };
    const spanish = join(root, entry.name, "locales", "es.json");
    if (existsSync(spanish)) {
      const text = json<{ description?: string }>(spanish).description;
      if (text) description.es = text;
    }
    listed.push({
      name: manifest.name,
      folder: entry.name,
      category: categoryOf({ kind: manifest.kind ?? null, tags: manifest.tags ?? [] }),
      color: manifest.color ?? "#34363b",
      description,
      otherNames: manifest.other_names ?? [],
      reads: readsOf(manifest),
    });
  }
  return listed.sort((a, b) => a.name.localeCompare(b.name));
}

/** Sorted presences grouped by category, in the order the categories first appear. */
export function byCategory(presences: Listed[]): [string, Listed[]][] {
  const groups = new Map<string, Listed[]>();
  for (const presence of presences) {
    groups.set(presence.category, [...(groups.get(presence.category) ?? []), presence]);
  }
  return [...groups];
}

/** One presence of each category for a short list, taking `preferred` names first. */
export function oneOfEach(presences: Listed[], preferred: string[]): Listed[] {
  return byCategory(presences).map(
    ([, group]) => group.find((presence) => preferred.includes(presence.name)) ?? group[0],
  );
}
