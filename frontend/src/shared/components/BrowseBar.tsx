import { CATEGORIES, PAGE_SIZES, rangeOf, type Category, type Filter } from "../browse";
import { useT } from "../i18n";

interface Props {
  filter: Filter;
  counts: Record<Category, number>;
  favoriteCount: number;
  onChange: (filter: Filter) => void;
}

function Chip({ active, label, count, onClick }: { active: boolean; label: string; count: number; onClick: () => void }) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      style={{ minHeight: 0 }}
      className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs ${
        active
          ? "border-neutral-600 bg-white/[0.08] text-neutral-100"
          : "border-neutral-800 text-neutral-400 hover:text-neutral-100 hover:bg-white/[0.04]"
      }`}
    >
      {label}
      <span className="tabular-nums text-neutral-500">{count}</span>
    </button>
  );
}

/** Search box, category chips and the favorites switch above a list. */
export default function BrowseBar({ filter, counts, favoriteCount, onChange }: Props) {
  const { t } = useT();
  const total = CATEGORIES.reduce((sum, category) => sum + counts[category], 0);
  return (
    <div className="mb-4 space-y-3">
      <input
        type="search"
        value={filter.text}
        placeholder={t("browse.search")}
        aria-label={t("browse.search")}
        className="w-full rounded-md border border-neutral-800 bg-neutral-900 px-3 py-2 text-[13px] text-neutral-100 placeholder:text-neutral-600"
        onInput={(event) => onChange({ ...filter, text: (event.target as HTMLInputElement).value })}
      />
      <div className="flex flex-wrap gap-1.5" role="group" aria-label={t("browse.categories")}>
        <Chip
          active={filter.category === "" && !filter.favoritesOnly}
          label={t("browse.all")}
          count={total}
          onClick={() => onChange({ ...filter, category: "", favoritesOnly: false })}
        />
        <Chip
          active={filter.favoritesOnly}
          label={`★ ${t("browse.favorites")}`}
          count={favoriteCount}
          onClick={() => onChange({ ...filter, favoritesOnly: !filter.favoritesOnly })}
        />
        {CATEGORIES.filter((category) => counts[category] > 0).map((category) => (
          <Chip
            key={category}
            active={filter.category === category}
            label={t(`category.${category}`)}
            count={counts[category]}
            onClick={() => onChange({ ...filter, category: filter.category === category ? "" : category })}
          />
        ))}
      </div>
    </div>
  );
}

interface PagerProps {
  page: number;
  pages: number;
  /** Items per page. */
  size: number;
  /** Items in the whole list. */
  total: number;
  onPage: (page: number) => void;
  onSize: (size: number) => void;
}

/**
 * Under a list: items per page, which items are shown, previous and next.
 * It stays on a list that fits one page, so a smaller size can still be chosen.
 */
export function Pager({ page, pages, size, total, onPage, onSize }: PagerProps) {
  const { t } = useT();
  if (total === 0) return null;
  const button =
    "rounded-md border border-neutral-800 px-3 py-1 text-xs text-neutral-300 hover:bg-white/[0.04] disabled:opacity-40";
  return (
    <nav className="mt-4 flex flex-wrap items-center justify-between gap-x-4 gap-y-2" aria-label={t("browse.pages")}>
      <div className="flex items-center gap-3 text-xs tabular-nums text-neutral-500">
        <label className="flex items-center gap-2">
          {t("browse.perPage")}
          <select
            value={size}
            style={{ minHeight: 0 }}
            className="rounded-md border border-neutral-800 bg-neutral-900 px-2 py-1 text-xs text-neutral-300"
            onChange={(event) => onSize(Number((event.target as HTMLSelectElement).value))}
          >
            {PAGE_SIZES.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        </label>
        <span>{t("browse.range", { ...rangeOf(total, page, size), total })}</span>
      </div>
      <div className="flex items-center gap-2">
        <button type="button" className={button} disabled={page === 0} onClick={() => onPage(page - 1)}>
          {t("browse.previous")}
        </button>
        <span className="text-xs tabular-nums text-neutral-500">{t("browse.pageOf", { page: page + 1, pages })}</span>
        <button type="button" className={button} disabled={page >= pages - 1} onClick={() => onPage(page + 1)}>
          {t("browse.next")}
        </button>
      </div>
    </nav>
  );
}

/** A star that marks a presence as favorite. */
export function FavoriteButton({ name, on, onToggle }: { name: string; on: boolean; onToggle: () => void }) {
  const { t } = useT();
  return (
    <button
      type="button"
      aria-pressed={on}
      title={on ? t("favorites.remove") : t("favorites.add")}
      aria-label={t(on ? "favorites.removeNamed" : "favorites.addNamed", { name })}
      onClick={onToggle}
      style={{ minHeight: 0 }}
      className={`shrink-0 rounded p-1 text-base leading-none ${
        on ? "text-warning" : "text-neutral-600 hover:text-neutral-300"
      }`}
    >
      {on ? "★" : "☆"}
    </button>
  );
}
