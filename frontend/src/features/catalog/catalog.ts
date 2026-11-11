import { errorMessage } from "../../shared/errors.ts";
import type { CatalogItem, CatalogState } from "../../shared/types";

/** What the Add tab lists: presences not installed yet, and installed ones that need a step. */
export function toAdd(items: CatalogItem[]): CatalogItem[] {
  return items.filter((item) => item.state !== "installed");
}

/** Size in the unit a person reads: "12 KB" or "1.4 MB". */
export function sizeText(bytes: number): string {
  return bytes < 1024 * 1024 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/**
 * "1 install" or "12,345 installs" for a presence, in the language in use.
 *
 * @param installs Counts by presence name, as the application gives them.
 * @param tn The plural lookup of the interface.
 * @returns Null without a count above zero, so nothing is shown for it.
 */
export function installsText(
  installs: Record<string, number> | null | undefined,
  name: string,
  tag: string,
  tn: (key: string, count: number, values?: Record<string, string | number>) => string,
): string | null {
  // A name such as "constructor" must not find something inherited.
  const count = installs && Object.hasOwn(installs, name) ? installs[name] : null;
  if (typeof count !== "number" || !Number.isSafeInteger(count) || count <= 0) return null;
  return tn("catalog.installs", count, { n: count.toLocaleString(tag) });
}

/** The label for an item's state, or null for one not installed. */
export function stateKey(state: CatalogState): string | null {
  switch (state) {
    case "installed":
      return "catalog.installed";
    case "update":
      return "catalog.updateAvailable";
    case "modified":
      return "catalog.modified";
    default:
      return null;
  }
}

/** What the application says when GitHub limits this network (see rpp-catalog's `RateLimited`). */
const RATE_LIMITED = "GitHub is limiting downloads from this network";

/**
 * A catalog error in the interface's language. The limit GitHub puts on a
 * network gets its own text, since it is the one people can do something
 * about (wait); any other error is shown as the application gave it.
 */
export function catalogError(error: unknown, t: (key: string) => string): string {
  const text = errorMessage(error);
  return text.includes(RATE_LIMITED) ? t("catalog.rateLimited") : text;
}
