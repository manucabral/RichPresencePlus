import { useEffect, useState } from "preact/hooks";
import bundled from "../../locales/en.json";
import { getLanguage, setLanguage as saveLanguage } from "../../platform/api";
import { BASE, merged, normalize, pick, plural as pluralText, translate, type Table } from "./core";

/**
 * Translations of the interface. English is bundled with the code; every
 * other `locales/<tag>.json` is a separate chunk loaded when it is chosen, so
 * adding a language costs nothing until someone uses it.
 */
const loaders = import.meta.glob<Table>(["../../locales/*.json", "!../../locales/en.json"], { import: "default" });

/** Tags that have a translation file, English first. */
export const available: string[] = [
  BASE,
  ...Object.keys(loaders)
    .map((path) => /\/([^/]+)\.json$/.exec(path)?.[1] ?? "")
    .filter((tag) => tag !== "" && tag !== BASE && normalize(tag) === tag)
    .sort(),
];

let tag = BASE;
const english: Table = bundled;
let table: Table = english;
const listeners = new Set<() => void>();

function publish() {
  for (const listener of listeners) listener();
}

async function load(requested: string): Promise<void> {
  const chosen = pick(requested, available) ?? BASE;
  const key = Object.keys(loaders).find((path) => path.endsWith(`/${chosen}.json`));
  let next: Table = english;
  if (chosen !== BASE && key) {
    try {
      next = merged({ [BASE]: english, [chosen]: await loaders[key]() }, chosen);
    } catch {
      // A chunk that cannot load leaves the interface in English.
    }
  }
  tag = chosen;
  table = next;
  document.documentElement.lang = chosen;
  publish();
}

/** Looks a text up in the language in use. Prefer `useT()` inside components. */
export function t(key: string, values?: Record<string, string | number>): string {
  return translate(table, key, values);
}

/** Looks up the plural form of a text for a count. */
export function tn(key: string, count: number, values?: Record<string, string | number>): string {
  return pluralText(table, tag, key, count, values);
}

/** Language tag in use. */
export function current(): string {
  return tag;
}

/**
 * Reads the saved choice from the desktop side and loads that language.
 * The interface starts in English and switches when the file arrives.
 */
export async function startLanguage(): Promise<void> {
  try {
    const choice = await getLanguage();
    await load(choice.setting === "auto" ? choice.system : choice.setting);
  } catch {
    await load(navigator.language);
  }
}

/** Saves and applies a language: `auto` or a tag from `available`. */
export async function chooseLanguage(setting: string): Promise<void> {
  await saveLanguage(setting);
  const choice = await getLanguage();
  await load(choice.setting === "auto" ? choice.system : choice.setting);
}

/** `t` and `tn` bound to the language in use; the component re-renders when it changes. */
export function useT() {
  const [, bump] = useState(0);
  useEffect(() => {
    const listener = () => bump((n) => n + 1);
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }, []);
  return { t, tn, tag };
}
