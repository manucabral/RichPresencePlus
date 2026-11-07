import { useState } from "preact/hooks";
import { useParams } from "../../shared/router";
import BackLink from "../../shared/components/BackLink";
import { toast } from "../../shared/toast";
import { getCatalog, installPresence, removePresence, restorePresence } from "../../platform/api";
import { FavoriteButton } from "../../shared/components/BrowseBar";
import Button from "../../shared/components/Button";
import Logo from "../../shared/components/Logo";
import Tags from "../../shared/components/Tags";
import { Chip, Panel, Section, SettingRow, selectClass, pageWidth } from "../../shared/components/ui";
import { useFavorites } from "../../shared/favorites";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import type { Access, Grant } from "../../shared/types";
import { accessLines } from "../presences/access";
import { catalogError, sizeText, stateKey } from "./catalog";

/** One presence of the catalog: what it is, and installing, updating or going back. */
export default function CatalogDetailPage() {
  const { t } = useT();
  const { name = "" } = useParams();
  const catalog = useLoaded(() => getCatalog(false));
  const { favorites, toggle } = useFavorites();
  const [busy, setBusy] = useState(false);
  const [version, setVersion] = useState("");
  const [asked, setAsked] = useState<{ access: Access; grants: Grant[] } | null>(null);

  const back = <BackLink to="/catalog" label={t("catalog.title")} />;
  if (!catalog.data) return <p className="text-sm text-neutral-500">{catalog.error ? catalogError(catalog.error, t) : t("app.loading")}</p>;
  const item = catalog.data.items.find((entry) => entry.name === name);
  if (!item) {
    return (
      <div className={pageWidth}>
        {back}
        <p className="text-sm text-neutral-500">{t("catalog.notFound")}</p>
      </div>
    );
  }

  async function run(action: () => Promise<void>, failure: string) {
    setBusy(true);
    try {
      await action();
      catalog.reload();
    } catch (error) {
      toast(failure, { description: catalogError(error, t) });
    } finally {
      setBusy(false);
    }
  }

  const install = (accepted: Grant[]) =>
    run(async () => {
      const outcome = await installPresence(item.name, accepted);
      setAsked(outcome.state === "asks" ? outcome : null);
    }, t("catalog.installFailed"));

  const installed = item.state !== "available";
  // The list refreshes after each action, so the choice must still be valid.
  const chosen = item.history.includes(version) ? version : (item.history[0] ?? "");
  const main =
    item.state === "update" ? t("catalog.update") : item.state === "modified" ? t("catalog.reinstall") : t("catalog.install");

  return (
    <div className={pageWidth}>
      {back}
      <header className="mb-8 flex items-start gap-4">
        <Logo name={item.name} color={item.color} icon={item.icon} />
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-[22px] font-semibold tracking-tight text-neutral-100">{item.name}</h1>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-neutral-500">
            <span>v{item.version}</span>
            {item.kind && <Chip>{t(`kind.${item.kind}`)}</Chip>}
            {stateKey(item.state) && <Chip tone="accent">{t(stateKey(item.state)!)}</Chip>}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <FavoriteButton name={item.name} on={favorites.has(item.name)} onToggle={() => toggle(item.name)} />
          {item.state !== "installed" && (
            <Button
              variant="primary"
              loading={busy}
              onClick={() => install([])}
            >
              {main}
            </Button>
          )}
          {installed && (
            <Button
              variant="ghost"
              disabled={busy}
              onClick={() => run(() => removePresence(item.name), t("catalog.removeFailed"))}
            >
              {t("catalog.remove")}
            </Button>
          )}
        </div>
      </header>

      {asked && (
        <Section title={t("catalog.asksTitle")} hint={t("catalog.asksHint")}>
          <Panel>
            {accessLines(asked.access).map((line) => (
              <p key={line} className="px-4 py-3 text-[13px] text-neutral-200">
                {line}
              </p>
            ))}
          </Panel>
          <div className="mt-3 flex gap-2">
            <Button variant="primary" loading={busy} onClick={() => install(asked.grants)}>
              {t("catalog.asksAccept")}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={() => setAsked(null)}>
              {t("catalog.asksLater")}
            </Button>
          </div>
        </Section>
      )}

      <Section title={t("presences.about")}>
        <p className="text-[13px] leading-relaxed text-neutral-300">{item.description ?? t("presences.noDescription")}</p>
        <Tags author={item.author} tags={item.tags} />
      </Section>

      <Section title={t("catalog.details")}>
        <Panel>
          <SettingRow title={t("catalog.version")}>
            <span className="text-[13px] text-neutral-300">{item.version}</span>
          </SettingRow>
          {item.installed_version && item.installed_version !== item.version && (
            <SettingRow title={t("catalog.installedVersion")}>
              <span className="text-[13px] text-neutral-300">{item.installed_version}</span>
            </SettingRow>
          )}
          <SettingRow title={t("catalog.size")}>
            <span className="text-[13px] text-neutral-300">{sizeText(item.size)}</span>
          </SettingRow>
        </Panel>
        {installed && <p className="mt-2 text-xs text-neutral-500">{t("catalog.settingsHint")}</p>}
      </Section>

      {item.history.length > 0 && (
        <Section title={t("catalog.history")} hint={t("catalog.historyHint")}>
          <Panel>
            <SettingRow title={t("catalog.restoreVersion")}>
              <div className="flex items-center gap-2">
                <select
                  value={chosen}
                  aria-label={t("catalog.restoreVersion")}
                  className={selectClass}
                  onChange={(event) => setVersion((event.target as HTMLSelectElement).value)}
                >
                  {item.history.map((entry) => (
                    <option key={entry} value={entry}>
                      {entry}
                    </option>
                  ))}
                </select>
                <Button
                  variant="ghost"
                  className="text-xs"
                  disabled={busy || chosen === ""}
                  onClick={() => run(() => restorePresence(item.name, chosen), t("catalog.restoreFailed"))}
                >
                  {t("catalog.restore")}
                </Button>
              </div>
            </SettingRow>
          </Panel>
        </Section>
      )}
    </div>
  );
}
