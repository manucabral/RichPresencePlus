import { useState } from "preact/hooks";
import { Link } from "../../shared/router";
import { toast } from "../../shared/toast";
import { getDevStatus, openPresenceFolder, watchPresence } from "../../platform/api";
import Button from "../../shared/components/Button";
import { Panel, Section, SettingRow } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";

/**
 * Tools for a presence the user wrote: open its folder and watch it, so each
 * save is applied at once. Bundled and catalog presences cannot be edited and
 * do not get this section.
 */
export default function DevTools({ name }: { name: string }) {
  const { t } = useT();
  const status = useLoaded(getDevStatus, 1500);
  const [busy, setBusy] = useState(false);
  const watching = status.data?.watching ?? null;
  const mine = watching === name;

  async function openFolder() {
    try {
      await openPresenceFolder(name);
    } catch (error) {
      toast(t("presences.folderFailed"), { description: errorMessage(error) });
    }
  }

  async function toggleWatch() {
    setBusy(true);
    try {
      // Only one presence is watched at a time; watching this one ends the other.
      await watchPresence(mine ? null : name);
      status.reload();
    } catch (error) {
      toast(t("developer.watchFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  const reloaded = mine ? status.data?.last_reload : null;
  const hint = mine
    ? reloaded
      ? t("developer.lastReload", { time: new Date(reloaded * 1000).toLocaleTimeString() })
      : t("presences.watchingThis")
    : watching
      ? t("presences.watchingOther", { name: watching })
      : t("presences.watchHint");

  return (
    <Section title={t("presences.develop")}>
      <Panel>
        <SettingRow title={t("presences.folder")} hint={t("presences.folderHint")}>
          <Button variant="ghost" className="text-xs" onClick={openFolder}>
            {t("developer.openFolder")}
          </Button>
        </SettingRow>
        <SettingRow title={t("presences.watch")} hint={hint}>
          <Button
            variant={mine ? "warning" : "ghost"}
            className="text-xs"
            loading={busy}
            disabled={!status.data}
            onClick={toggleWatch}
          >
            {mine ? t("developer.stop") : t("developer.watch")}
          </Button>
        </SettingRow>
        {mine && status.data?.error && (
          <p className="break-words px-4 py-2.5 text-xs text-danger">{status.data.error}</p>
        )}
        <SettingRow title={t("presences.inspect")} hint={t("presences.inspectHint")}>
          <Link
            to={`/developer?presence=${encodeURIComponent(name)}`}
            className="inline-flex min-h-[34px] items-center whitespace-nowrap rounded-md border border-neutral-800 px-3.5 text-xs font-medium text-neutral-200 hover:bg-white/[0.04]"
          >
            {t("presences.inspectOpen")}
          </Link>
        </SettingRow>
      </Panel>
    </Section>
  );
}
