/**
 * How many times each presence was installed from the catalog, asked for when
 * the site is built. The numbers come from stats.json, which a daily workflow
 * writes on the catalog-stats branch of the catalog repository; the site is built
 * again each day after it.
 */

const STATS = "https://raw.githubusercontent.com/manucabral/RichPresencePlus-catalog/catalog-stats/stats.json";

/** Installs by presence folder, from the stable channel; anything malformed is left out. */
export function installsOf(stats: unknown): Map<string, number> {
  const installs = new Map<string, number>();
  const stable = (stats as { stable?: unknown } | null)?.stable;
  if (!Array.isArray(stable)) return installs;
  for (const entry of stable) {
    const { folder, installs: count } = (entry ?? {}) as { folder?: unknown; installs?: unknown };
    if (typeof folder === "string" && Number.isSafeInteger(count) && (count as number) > 0) installs.set(folder, count as number);
  }
  return installs;
}

let asked: Promise<Map<string, number>> | null = null;

/** Asks for the numbers once per build; without them the pages show none. */
export function loadInstalls(ask: typeof fetch = fetch): Promise<Map<string, number>> {
  asked ??= ask(STATS, { signal: AbortSignal.timeout(5000) })
    .then(async (response) => (response.ok ? installsOf(await response.json()) : new Map<string, number>()))
    .catch(() => new Map<string, number>());
  return asked;
}

/** "1 install" or "1,234 installs", in the page's words and number format. */
export function installsText(count: number, one: string, many: string, locale: string): string {
  return count === 1 ? one : many.replace("{count}", count.toLocaleString(locale));
}
