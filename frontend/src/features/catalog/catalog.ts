import { errorMessage } from "../../shared/errors.ts";
import type { CatalogState } from "../../shared/types";

/** Size in the unit a person reads: "12 KB" or "1.4 MB". */
export function sizeText(bytes: number): string {
  return bytes < 1024 * 1024 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
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
