import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { compareVersion, getVersions, restoreVersion, saveVersion } from "../../platform/api";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import type { FileChange, VersionView } from "../../shared/types";
import { lineSummary, sizeText } from "./workbench";

const field =
  "min-w-0 flex-1 rounded-md border border-neutral-800 bg-neutral-950 px-3 py-1.5 text-[13px] text-neutral-100 placeholder:text-neutral-600";

interface Compared {
  id: number;
  changes: FileChange[];
}

function Row({ version, name, onRestored }: { version: VersionView; name: string; onRestored: () => void }) {
  const { t } = useT();
  const [compared, setCompared] = useState<Compared | null>(null);
  const [confirming, setConfirming] = useState(false);

  async function compare() {
    if (compared) {
      setCompared(null);
      return;
    }
    try {
      setCompared({ id: version.id, changes: await compareVersion(name, version.id) });
    } catch (error) {
      toast(t("developer.compareFailed"), { description: errorMessage(error) });
    }
  }

  async function restore() {
    setConfirming(false);
    try {
      await restoreVersion(name, version.id);
      setCompared(null);
      toast(t("developer.restored", { id: version.id }));
      onRestored();
    } catch (error) {
      toast(t("developer.restoreFailed"), { description: errorMessage(error) });
    }
  }

  return (
    <li className="px-3 py-2 space-y-1.5">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="text-xs text-neutral-200">
            #{version.id}, {version.version}
            <span className="text-neutral-500">
              , {new Date(version.at * 1000).toLocaleString()}, {sizeText(version.size)}
            </span>
          </p>
          {version.notes && <p className="text-xs text-neutral-400 break-words">{version.notes}</p>}
        </div>
        <div className="flex items-center gap-1.5">
          <Button variant="ghost" className="text-xs" aria-expanded={compared !== null} onClick={compare}>
            {t("developer.compare")}
          </Button>
          {confirming ? (
            <Button variant="warning" className="text-xs" onClick={restore}>
              {t("developer.restoreConfirm")}
            </Button>
          ) : (
            <Button variant="ghost" className="text-xs" onClick={() => setConfirming(true)}>
              {t("developer.restore")}
            </Button>
          )}
        </div>
      </div>
      {compared &&
        (compared.changes.length === 0 ? (
          <p className="text-xs text-neutral-500">{t("developer.noChanges")}</p>
        ) : (
          <ul className="text-[11px] space-y-0.5">
            {compared.changes.map((change) => (
              <li key={change.path} className="flex justify-between gap-3">
                <span className="min-w-0 truncate text-neutral-300">
                  {t(`developer.change.${change.change}`)}: {change.path}
                </span>
                <span className="shrink-0 tabular-nums text-neutral-500">{lineSummary(change)}</span>
              </li>
            ))}
          </ul>
        ))}
    </li>
  );
}

/** Saved versions of a presence. */
export default function VersionsPanel({ name }: { name: string }) {
  const { t } = useT();
  const versions = useLoaded(() => getVersions(name));
  const [notes, setNotes] = useState("");
  const [busy, setBusy] = useState(false);

  async function save() {
    setBusy(true);
    try {
      await saveVersion(name, notes);
      setNotes("");
      versions.reload();
    } catch (error) {
      toast(t("developer.versionFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  const list = versions.data ?? [];
  return (
    <div className="space-y-2">
      <p className="text-xs text-neutral-500">{t("developer.versionsHint")}</p>
      <div className="flex flex-wrap items-center gap-2">
        <input
          value={notes}
          maxLength={500}
          className={field}
          placeholder={t("developer.notes")}
          aria-label={t("developer.notes")}
          onInput={(event) => setNotes((event.target as HTMLInputElement).value)}
        />
        <Button variant="neutral" className="text-xs" disabled={busy} onClick={save}>
          {t("developer.saveVersion")}
        </Button>
      </div>
      {versions.error && <p className="text-xs text-danger break-words">{versions.error}</p>}
      {list.length > 0 && (
        <ul className="max-h-64 overflow-auto divide-y divide-neutral-800/70 rounded-lg border border-neutral-800">
          {list.map((version) => (
            <Row key={version.id} version={version} name={name} onRestored={versions.reload} />
          ))}
        </ul>
      )}
    </div>
  );
}
