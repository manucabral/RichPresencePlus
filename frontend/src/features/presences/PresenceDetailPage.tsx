import { useState } from "preact/hooks";
import { Link, useParams } from "../../shared/router";
import { toast } from "../../shared/toast";
import { getIcons, getPreferences, getPresences, setPresenceEnabled } from "../../platform/api";
import BackLink from "../../shared/components/BackLink";
import { FavoriteButton } from "../../shared/components/BrowseBar";
import Logo from "../../shared/components/Logo";
import Switch from "../../shared/components/Switch";
import Tags from "../../shared/components/Tags";
import { Chip, Panel, Section, pageWidth } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useFavorites } from "../../shared/favorites";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { isExcluded } from "../../shared/presence";
import { accessLines } from "./access";
import DevTools from "./DevTools";
import DisplaySettings from "./DisplaySettings";

/** One presence: what it does, what it can see, and how it shows on Discord. */
export default function PresenceDetailPage() {
  const { t } = useT();
  const { name = "" } = useParams();
  const presences = useLoaded(getPresences, 3000);
  const rules = useLoaded(getPreferences).data?.global;
  const icons = useLoaded(getIcons).data ?? {};
  const { favorites, toggle: toggleFavorite } = useFavorites();
  const [busy, setBusy] = useState(false);

  const back = <BackLink to="/presences" label={t("presences.title")} />;
  if (!presences.data) {
    return <p className="text-sm text-neutral-500">{presences.error ?? t("app.loading")}</p>;
  }
  const presence = presences.data.find((entry) => entry.name === name);
  if (!presence) {
    return (
      <div className={pageWidth}>
        {back}
        <p className="text-sm text-neutral-500">{t("presences.notFound")}</p>
      </div>
    );
  }

  async function toggle(enabled: boolean) {
    setBusy(true);
    try {
      await setPresenceEnabled(presence!.name, enabled);
      presences.reload();
    } catch (error) {
      toast(t("presences.changeFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  const lines = accessLines(presence.access);
  return (
    <div className={pageWidth}>
      {back}
      <header className="mb-8 flex items-start gap-4">
        <Logo name={presence.name} color={presence.color} icon={presence.icon ?? icons[presence.name] ?? null} />
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-[22px] font-semibold tracking-tight text-neutral-100">{presence.name}</h1>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-neutral-500">
            <span>v{presence.version}</span>
            {presence.kind && <Chip>{t(`kind.${presence.kind}`)}</Chip>}
            <Chip tone={presence.origin === "local" ? "accent" : "neutral"} title={t(`origin.${presence.origin}.hint`)}>
              {t(`origin.${presence.origin}`)}
            </Chip>
            <span className="inline-flex items-center gap-1.5">
              <span className={`h-1.5 w-1.5 rounded-full ${presence.running ? "bg-accent" : "bg-neutral-600"}`} />
              {presence.running ? t("presences.running") : t("presences.stopped")}
            </span>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <FavoriteButton name={presence.name} on={favorites.has(presence.name)} onToggle={() => toggleFavorite(presence.name)} />
          <Switch
            checked={presence.enabled}
            onChange={toggle}
            label={t("presences.toggle", { name: presence.name })}
            disabled={busy}
          />
        </div>
      </header>

      {rules && isExcluded(presence, rules) && (
        <p className="-mt-4 mb-6 rounded-lg border border-neutral-800 bg-neutral-900/60 px-4 py-3 text-[13px] text-neutral-300">
          {t("presences.excluded")}{" "}
          <Link to="/privacy" className="text-accent-soft underline underline-offset-2 hover:text-accent-hover">
            {t("presences.excludedLink")}
          </Link>
        </p>
      )}

      <Section title={t("presences.about")}>
        <p className="text-[13px] leading-relaxed text-neutral-300">
          {presence.description ?? t("presences.noDescription")}
        </p>
        <Tags author={presence.author} tags={presence.tags} />
        <p className="mt-2 text-xs text-neutral-500">{t("presences.interval", { seconds: presence.interval })}</p>
      </Section>

      <Section title={t("presences.access")} hint={t("presences.accessHint")}>
        <Panel>
          {lines.length === 0 ? (
            <p className="px-4 py-3 text-[13px] text-neutral-400">{t("presences.accessNone")}</p>
          ) : (
            lines.map((line) => (
              <p key={line} className="break-words px-4 py-2.5 text-[13px] text-neutral-300">
                {line}
              </p>
            ))
          )}
        </Panel>
      </Section>

      <DisplaySettings presence={presence} onSaved={presences.reload} />

      {presence.editable && <DevTools name={presence.name} />}
    </div>
  );
}
