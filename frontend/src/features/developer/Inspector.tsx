import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { inspectPresence, saveTestCase } from "../../platform/api";
import ActivityCard from "../../shared/components/ActivityCard";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";

interface Props {
  name: string;
  /** Run again every couple of seconds, as when the presence is being watched. */
  live: boolean;
  /** Called after keeping what was seen as a test case; none for a presence of someone else. */
  onCaseSaved?: () => void;
}

function Block({ title, value }: { title: string; value: unknown }) {
  const { t } = useT();
  const text = value === null || value === undefined ? t("developer.nothing") : JSON.stringify(value, null, 2);
  return (
    <div>
      <p className="mb-1 text-xs text-neutral-500">{title}</p>
      <pre className="max-h-56 overflow-auto rounded-lg border border-neutral-800 bg-neutral-950 p-3 text-[11px] leading-relaxed text-neutral-300 whitespace-pre-wrap break-words">
        {text}
      </pre>
    </div>
  );
}

/** What a presence reads and would show, from running it once or on a timer. */
export default function Inspector({ name, live, onCaseSaved }: Props) {
  const { t } = useT();
  const run = useLoaded(() => inspectPresence(name), live ? 2000 : 0);
  const seen = run.data;
  const [caseName, setCaseName] = useState("");

  async function keep() {
    if (!seen) return;
    try {
      await saveTestCase(name, { name: caseName, snapshot: seen.snapshot, expected: seen.activity });
      setCaseName("");
      onCaseSaved?.();
    } catch (error) {
      toast(t("developer.casesFailed"), { description: errorMessage(error) });
    }
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm text-neutral-300">{t("developer.inspect")}</h3>
        {!live && (
          <Button variant="neutral" className="text-xs" onClick={run.reload}>
            {t("developer.runOnce")}
          </Button>
        )}
      </div>
      {run.error && <p className="text-xs text-danger break-words">{run.error}</p>}
      {seen?.error && <p className="text-xs text-danger break-words">{seen.error}</p>}
      {seen && onCaseSaved && seen.snapshot !== null && !seen.error && (
        <div className="flex flex-wrap items-center gap-2">
          <input
            value={caseName}
            maxLength={60}
            className="min-w-0 flex-1 rounded-md border border-neutral-800 bg-neutral-950 px-3 py-1.5 text-[13px] text-neutral-100 placeholder:text-neutral-600"
            placeholder={t("developer.caseName")}
            aria-label={t("developer.caseName")}
            onInput={(event) => setCaseName((event.target as HTMLInputElement).value)}
          />
          <Button variant="neutral" className="text-xs" disabled={caseName.trim() === ""} onClick={keep}>
            {t("developer.caseSave")}
          </Button>
        </div>
      )}
      {seen && (
        <div className="grid gap-3 md:grid-cols-2">
          <div className="space-y-3">
            <Block title={t("developer.snapshot")} value={seen.snapshot} />
            <Block title={t("developer.meta")} value={seen.activity ? seen.meta : null} />
          </div>
          <div className="space-y-3">
            <div>
              <p className="mb-1 text-xs text-neutral-500">{t("developer.activity")}</p>
              <ActivityCard title={t("manual.previewTitle")} activity={seen.activity} />
            </div>
            {seen.console.length > 0 && (
              <div>
                <p className="mb-1 text-xs text-neutral-500">{t("developer.console")}</p>
                <pre className="max-h-40 overflow-auto rounded-lg border border-neutral-800 bg-neutral-950 p-3 text-[11px] text-neutral-400 whitespace-pre-wrap break-words">
                  {seen.console.join("\n")}
                </pre>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
