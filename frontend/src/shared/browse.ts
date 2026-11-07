/**
 * Searching, filtering and paging the lists of presences (installed ones and
 * the catalog). Pure functions, so both pages behave the same and are tested
 * without a window.
 */
import { matches } from "./presence.ts";

/** Categories of the lists. */
export const CATEGORIES = ["watch", "listen", "play", "create", "more"] as const;
export type Category = (typeof CATEGORIES)[number];

/** Items per page unless the user chooses another size. */
export const PAGE_SIZE = 10;

/** The sizes offered under a list. */
export const PAGE_SIZES = [5, 10, 25, 50] as const;

/** A remembered page size; anything that is not on offer gives the default. */
export function readPageSize(stored: string | null): number {
  const size = Number(stored);
  return (PAGE_SIZES as readonly number[]).includes(size) ? size : PAGE_SIZE;
}

/** What a list item needs to be searched and grouped. */
export interface Browsable {
  name: string;
  description: string | null;
  kind: string | null;
  author: string | null;
  tags: string[];
}

/** Tags of presences for making something, which `kind` cannot say. */
const MAKING = new Set(["code", "design"]);

/**
 * The category of an item: what someone is doing while it shows.
 */
export function categoryOf(item: Pick<Browsable, "kind" | "tags">): Category {
  if (item.tags.some((tag) => MAKING.has(tag.toLowerCase()))) return "create";
  switch (item.kind) {
    case "game":
      return "play";
    case "video":
      return "watch";
    case "music":
      return "listen";
    default:
      return "more";
  }
}

/** What the user chose to narrow the list. */
export interface Filter {
  text: string;
  /** Empty for every category. */
  category: Category | "";
  favoritesOnly: boolean;
}

export const NO_FILTER: Filter = { text: "", category: "", favoritesOnly: false };

/** The items that pass the filter, favorites first, then by name. */
export function browse<T extends Browsable>(items: readonly T[], filter: Filter, favorites: ReadonlySet<string>): T[] {
  return items
    .filter(
      (item) =>
        (filter.category === "" || categoryOf(item) === filter.category) &&
        (!filter.favoritesOnly || favorites.has(item.name)) &&
        matches(item, filter.text, ""),
    )
    .sort(
      (a, b) =>
        Number(favorites.has(b.name)) - Number(favorites.has(a.name)) || a.name.localeCompare(b.name),
    );
}

/** How many items each category holds, to show next to its name. */
export function countByCategory(items: readonly Browsable[]): Record<Category, number> {
  const counts = Object.fromEntries(CATEGORIES.map((category) => [category, 0])) as Record<Category, number>;
  for (const item of items) counts[categoryOf(item)] += 1;
  return counts;
}

/** One page of a list. */
export interface Page<T> {
  items: T[];
  /** The page shown, from 0; kept inside the list when it shrinks. */
  page: number;
  /** At least 1, so "1 of 1" reads right for an empty list. */
  pages: number;
}

export function paginate<T>(items: readonly T[], page: number, size = PAGE_SIZE): Page<T> {
  const pages = Math.max(1, Math.ceil(items.length / size));
  const shown = Math.min(Math.max(0, Math.floor(page)), pages - 1);
  return { items: items.slice(shown * size, shown * size + size), page: shown, pages };
}

/**
 * The items a page shows, counted from 1, for "1–10 of 23".
 *
 * @returns `first` and `last`, both 0 for an empty list.
 */
export function rangeOf(total: number, page: number, size = PAGE_SIZE): { first: number; last: number } {
  if (total === 0) return { first: 0, last: 0 };
  return { first: page * size + 1, last: Math.min(total, (page + 1) * size) };
}

/** The page that holds the item at `index`. */
export function pageOf(index: number, size = PAGE_SIZE): number {
  return index < 0 ? 0 : Math.floor(index / size);
}
