import { useMemo, useState } from "preact/hooks";
import { Link } from "../../shared/router";
import { getCatalog, getIcons } from "../../platform/api";
import BrowseBar, { FavoriteButton, Pager } from "../../shared/components/BrowseBar";
import InfoCard from "../../shared/components/InfoCard";
import Logo from "../../shared/components/Logo";
import PageTitle from "../../shared/components/PageTitle";
import { Chip, pageWidth } from "../../shared/components/ui";
import { NO_FILTER, browse, countByCategory, paginate, type Filter } from "../../shared/browse";
import { usePaging } from "../../shared/paging";
import { useFavorites } from "../../shared/favorites";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import type { CatalogItem } from "../../shared/types";
import { catalogPath } from "../../shared/navigation";
import { catalogError, stateKey } from "./catalog";

/** One line per item; installing and versions are on its own page. */
interface RowProps {
  item: CatalogItem;
  /** The logo file of a presence that came with the app. */
  icon: string | null;
  favorite: boolean;
  onFavorite: () => void;
}

function CatalogRow({ item, icon, favorite, onFavorite }: RowProps) {
  const { t } = useT();
  const state = stateKey(item.state);
  return (
    <li className="flex items-center gap-3 px-4 py-2.5 hover:bg-white/[0.025]">
      <Link to={catalogPath(item.name)} className="flex min-w-0 flex-1 items-center gap-3">
        <Logo name={item.name} color={item.color} icon={item.icon ?? icon} />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[13px] font-medium text-neutral-100">{item.name}</span>
          {item.description && <span className="block truncate text-xs text-neutral-500">{item.description}</span>}
        </span>
        {state && (
          <Chip tone={item.state === "installed" ? "neutral" : "accent"}>{t(state)}</Chip>
        )}
      </Link>
      <FavoriteButton name={item.name} on={favorite} onToggle={onFavorite} />
    </li>
  );
}

export default function CatalogPage() {
  const { t } = useT();
  const catalog = useLoaded(() => getCatalog(false));
  // Logos are files of each presence; the ones that came with the app are known here.
  const icons = useLoaded(getIcons).data ?? {};
  const { favorites, toggle } = useFavorites();
  const [filter, setFilter] = useState<Filter>(NO_FILTER);
  const paging = usePaging();
  const data = catalog.data;
  const items = data?.items ?? [];
  const found = useMemo(() => browse(items, filter, favorites), [items, filter, favorites]);
  const counts = useMemo(() => countByCategory(items), [items]);

  if (!data) return <p className="text-sm text-neutral-500">{catalog.error ? catalogError(catalog.error, t) : t("app.loading")}</p>;

  const shown = paginate(found, paging.page, paging.size);
  const change = (next: Filter) => {
    setFilter(next);
    paging.setPage(0);
  };

  return (
    <div className={pageWidth}>
      <PageTitle title={t("catalog.title")} subtitle={t("catalog.subtitle")} />
      {!data.available ? (
        <InfoCard text={t("catalog.unavailable")} />
      ) : (
        <>
          <BrowseBar
            filter={filter}
            counts={counts}
            favoriteCount={items.filter((item) => favorites.has(item.name)).length}
            onChange={change}
          />
          {shown.items.length === 0 ? (
            <p className="text-sm text-neutral-500">{t("catalog.none")}</p>
          ) : (
            <ul className="divide-y divide-neutral-800/70 overflow-hidden rounded-lg border border-neutral-800 bg-neutral-900/40">
              {shown.items.map((item) => (
                <CatalogRow
                  key={item.name}
                  item={item}
                  icon={icons[item.name] ?? null}
                  favorite={favorites.has(item.name)}
                  onFavorite={() => toggle(item.name)}
                />
              ))}
            </ul>
          )}
          <Pager
            page={shown.page}
            pages={shown.pages}
            size={paging.size}
            total={found.length}
            onPage={paging.setPage}
            onSize={paging.setSize}
          />
        </>
      )}
    </div>
  );
}
