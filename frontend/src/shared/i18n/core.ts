/**
 * Choosing a language and filling in values. No framework and no I/O, so the
 * rules can be tested in Node. They mirror `rpp-core/src/locale.rs`, which
 * does the same for presences.
 */

/** Keys and texts of one language. */
export type Table = Record<string, string>;

/** Language every table falls back to. */
export const BASE = "en";

/** Normalizes a language tag such as `es_ar` or `EN` to `es-AR` or `en`; null if it is not a tag. */
export function normalize(tag: string): string | null {
  const parts = tag.trim().split(".")[0].split(/[-_]/);
  const language = parts[0];
  if (!/^[A-Za-z]{2,3}$/.test(language)) return null;
  let out = language.toLowerCase();
  for (const part of parts.slice(1)) {
    if (!/^[A-Za-z0-9]+$/.test(part)) return null;
    out += "-" + (part.length === 2 ? part.toUpperCase() : part.toLowerCase());
  }
  return out;
}

/**
 * Picks the best available language for a request: the exact tag, the same
 * language without region, any regional variant, then the base language.
 * Null if not even the base exists.
 */
export function pick(requested: string, available: readonly string[]): string | null {
  const wanted = normalize(requested);
  if (wanted) {
    if (available.includes(wanted)) return wanted;
    const language = wanted.split("-")[0];
    if (available.includes(language)) return language;
    const variant = available.find((tag) => tag.startsWith(language + "-"));
    if (variant) return variant;
  }
  return available.includes(BASE) ? BASE : null;
}

/**
 * Replaces `{name}` placeholders. A placeholder with no value stays as
 * written, so a missing value is visible instead of silently dropped.
 */
export function fill(text: string, values?: Record<string, string | number>): string {
  if (!values) return text;
  return text.replace(/\{([A-Za-z0-9_]+)\}/g, (whole, name: string) =>
    Object.prototype.hasOwnProperty.call(values, name) ? String(values[name]) : whole,
  );
}

/** Looks a key up and fills it. A key with no text comes back as written. */
export function translate(table: Table, key: string, values?: Record<string, string | number>): string {
  return fill(table[key] ?? key, values);
}

/**
 * Looks up the plural form of a key: `key_one`, `key_other` and so on, as
 * the language's plural rules choose; `key_other` is the fallback. The count
 * is also available as `{count}`.
 */
export function plural(
  table: Table,
  tag: string,
  key: string,
  count: number,
  values?: Record<string, string | number>,
): string {
  const category = new Intl.PluralRules(tag).select(count);
  const text = table[`${key}_${category}`] ?? table[`${key}_other`] ?? key;
  return fill(text, { ...values, count });
}

/** The base language overlaid with the chosen one, which must be a key of `tables`. */
export function merged(tables: Record<string, Table>, tag: string): Table {
  return { ...(tables[BASE] ?? {}), ...(tag === BASE ? {} : (tables[tag] ?? {})) };
}

/**
 * Checks a translation against the base language. Used by the tool that
 * reviews contributions. One message per problem; empty when it is sound.
 */
export function review(base: Table, other: Table): string[] {
  const names = (text: string) => [...text.matchAll(/\{([A-Za-z0-9_]+)\}/g)].map((m) => m[1]).sort();
  const problems: string[] = [];
  for (const [key, text] of Object.entries(other)) {
    if (typeof text !== "string") {
      problems.push(`${key}: the value is not text`);
      continue;
    }
    if (text.trim() === "") problems.push(`${key}: the text is empty`);
    const original = base[key];
    if (original === undefined) {
      // Plural forms of other languages (few, many...) have no English twin.
      const stem = key.replace(/_(zero|one|two|few|many|other)$/, "");
      if (stem === key || base[`${stem}_other`] === undefined) problems.push(`${key}: no such key in English`);
      continue;
    }
    if (names(original).join() !== names(text).join()) {
      problems.push(`${key}: placeholders differ from English (${names(original).join(", ") || "none"})`);
    }
  }
  return problems;
}
