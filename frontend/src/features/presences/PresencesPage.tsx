import { useMemo, useState } from "preact/hooks";
import { Link } from "../../shared/router";
import { toast } from "../../shared/toast";
import { getIcons, getPreferences, getPresences, setPresenceEnabled } from "../../platform/api";
import { isExcluded } from "../../shared/presence";
import BrowseBar, { FavoriteButton, Pager } from "../../shared/components/BrowseBar";
import InfoCard from "../../shared/components/InfoCard";
import Logo from "../../shared/components/Logo";
import PageTitle from "../../shared/components/PageTitle";
import Switch from "../../shared/components/Switch";
import { Chip, pageWidth } from "../../shared/components/ui";
import { NO_FILTER, browse, countByCategory, paginate, type Filter } from "../../shared/browse";
import { errorMessage } from "../../shared/errors";
import { useFavorites } from "../../shared/favorites";
import { usePaging } from "../../shared/paging";
import ProgramCard from "./ProgramCard";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { presencePath } from "../../shared/navigation";
import type { PresenceView } from "../../shared/types";

interface RowProps {
  presence: PresenceView;
  icon: string | null;
  favorite: boolean;
  /** The privacy rules leave it with nothing to publish. */
  excluded: boolean;
  onFavorite: () => void;
  onChange: () => void;
}

/** One line per presence; everything else is on its own page. */
function PresenceRow({ presence, icon, favorite, excluded, onFavorite, onChange }: RowProps) {
  const { t } = useT();
  const [busy, setBusy] = useState(false);

  async function toggle(enabled: boolean) {
    setBusy(true);
    try {
      await setPresenceEnabled(presence.name, enabled);
      onChange();
    } catch (error) {
      toast(t("presences.changeFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <li className="flex items-center gap-3 px-4 py-2.5 hover:bg-white/[0.025]">
      <Link to={presencePath(presence.name)} className="flex min-w-0 flex-1 items-center gap-3">
        <Logo name={presence.name} color={presence.color} icon={icon} />
        <span className="min-w-0 flex-1">
          <span className="flex min-w-0 items-center gap-2">
            <span
              className={`h-1.5 w-1.5 shrink-0 rounded-full ${presence.running ? "bg-accent" : "bg-neutral-600"}`}
              title={presence.running ? t("presences.running") : t("presences.stopped")}
            />
            <span className="truncate text-[13px] font-medium text-neutral-100">{presence.name}</span>
          </span>
          {presence.description && (
            <span className="block truncate text-xs text-neutral-500">{presence.description}</span>
          )}
        </span>
        {excluded && <Chip title={t("presences.excluded")}>{t("presences.excludedTag")}</Chip>}
        {presence.kind && (
          <span className="hidden sm:inline">
            <Chip>{t(`kind.${presence.kind}`)}</Chip>
          </span>
        )}
      </Link>
      <FavoriteButton name={presence.name} on={favorite} onToggle={onFavorite} />
      <Switch checked={presence.enabled} onChange={toggle} label={t("presences.toggle", { name: presence.name })} disabled={busy} />
    </li>
  );
}

export default function PresencesPage() {
  const { t } = useT();
  const presences = useLoaded(getPresences, 3000);
  const rules = useLoaded(getPreferences).data?.global;
  // Logos kept as files change only when a folder does; one read per visit is enough.
  const icons = useLoaded(getIcons).data ?? {};
  const { favorites, toggle } = useFavorites();
  const [filter, setFilter] = useState<Filter>(NO_FILTER);
  const paging = usePaging();

  const all = presences.data ?? [];
  const found = useMemo(() => browse(all, filter, favorites), [all, filter, favorites]);
  const counts = useMemo(() => countByCategory(all), [all]);
  const shown = paginate(found, paging.page, paging.size);
  const change = (next: Filter) => {
    setFilter(next);
    paging.setPage(0);
  };

  return (
    <div className={pageWidth}>
      <PageTitle title={t("presences.title")} subtitle={t("presences.subtitle")} />
      {!presences.data ? (
        <p className="text-sm text-neutral-500">{presences.error ?? t("app.loading")}</p>
      ) : all.length === 0 ? (
        <p className="text-sm text-neutral-500">{t("presences.none")}</p>
      ) : (
        <>
          <ProgramCard onCreated={presences.reload} />
          <BrowseBar
            filter={filter}
            counts={counts}
            favoriteCount={all.filter((p) => favorites.has(p.name)).length}
            onChange={change}
          />
          {shown.items.length === 0 ? (
            <p className="text-sm text-neutral-500">{t("browse.none")}</p>
          ) : (
            <ul className="divide-y divide-neutral-800/70 overflow-hidden rounded-lg border border-neutral-800 bg-neutral-900/40">
              {shown.items.map((presence) => (
                <PresenceRow
                  key={presence.name}
                  presence={presence}
                  icon={presence.icon ?? icons[presence.name] ?? null}
                  favorite={favorites.has(presence.name)}
                  excluded={rules ? isExcluded(presence, rules) : false}
                  onFavorite={() => toggle(presence.name)}
                  onChange={presences.reload}
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
          <div className="mt-4">
            <InfoCard text={t("presences.request")} />
          </div>
        </>
      )}
    </div>
  );
}
