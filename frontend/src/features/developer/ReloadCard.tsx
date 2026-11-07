import { useState } from "preact/hooks";
import { reloadPresences } from "../../platform/api";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useT } from "../../shared/i18n";
import { toast } from "../../shared/toast";
import type { Problem } from "../../shared/types";

/** Reads the presence folders again and lists what could not load. */
export default function ReloadCard() {
  const { t } = useT();
  const [problems, setProblems] = useState<Problem[] | null>(null);
  const [busy, setBusy] = useState(false);

  async function reload() {
    setBusy(true);
    try {
      setProblems(await reloadPresences());
    } catch (error) {
      toast(t("developer.reloadFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <p className="text-xs text-neutral-500">{t("developer.reloadHint")}</p>
        <Button variant="neutral" loading={busy} onClick={reload}>
          {t("developer.reloadButton")}
        </Button>
      </div>
      {problems && problems.length === 0 && <p className="text-xs text-accent-soft">{t("developer.allLoaded")}</p>}
      {problems && problems.length > 0 && (
        <ul className="space-y-1">
          {problems.map((problem) => (
            <li key={`${problem.folder}${problem.message}`} className="text-xs">
              <span className="text-neutral-200">{problem.folder}</span>
              <span className="block text-danger break-words">{problem.message}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
