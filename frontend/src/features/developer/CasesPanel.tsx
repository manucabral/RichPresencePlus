import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { deleteTestCase, getTestCases, runTestCases, saveTestCase } from "../../platform/api";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import type { CaseResult, TestCase } from "../../shared/types";

function Shown({ title, value }: { title: string; value: unknown }) {
  return (
    <div className="min-w-0">
      <p className="mb-1 text-[11px] text-neutral-500">{title}</p>
      <pre className="max-h-40 overflow-auto rounded border border-neutral-800 bg-neutral-950 p-2 text-[11px] text-neutral-300 whitespace-pre-wrap break-words">
        {JSON.stringify(value, null, 2)}
      </pre>
    </div>
  );
}

interface RowProps {
  entry: TestCase;
  result: CaseResult | undefined;
  onAccept: (result: CaseResult) => void;
  onDelete: () => void;
}

function Row({ entry, result, onAccept, onDelete }: RowProps) {
  const { t } = useT();
  const failed = result !== undefined && !result.passed;
  return (
    <li className="px-3 py-2 space-y-2">
      <div className="flex items-center justify-between gap-2">
        <p className="min-w-0 truncate text-xs text-neutral-200">
          {result && (
            <span className={result.passed ? "text-accent-soft" : "text-danger"}>
              {t(result.passed ? "developer.casePassed" : "developer.caseFailed")}{" "}
            </span>
          )}
          {entry.name}
        </p>
        <div className="flex items-center gap-1.5">
          {failed && !result.error && (
            <Button variant="ghost" className="text-xs" onClick={() => onAccept(result)}>
              {t("developer.caseAccept")}
            </Button>
          )}
          <Button variant="ghost" className="text-xs" onClick={onDelete}>
            {t("developer.caseDelete")}
          </Button>
        </div>
      </div>
      {result?.error && <p className="text-xs text-danger break-words">{result.error}</p>}
      {failed && !result.error && (
        <div className="grid gap-2 md:grid-cols-2">
          <Shown title={t("developer.caseExpected")} value={result.expected} />
          <Shown title={t("developer.caseActual")} value={result.actual} />
        </div>
      )}
    </li>
  );
}

/** Snapshots kept to check a change without opening the site. */
export default function CasesPanel({ name }: { name: string }) {
  const { t } = useT();
  const cases = useLoaded(() => getTestCases(name));
  const [results, setResults] = useState<CaseResult[]>([]);
  const [busy, setBusy] = useState(false);

  async function run() {
    setBusy(true);
    try {
      setResults(await runTestCases(name));
    } catch (error) {
      toast(t("developer.casesFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  async function change(work: () => Promise<void>) {
    try {
      await work();
      setResults([]);
      cases.reload();
    } catch (error) {
      toast(t("developer.casesFailed"), { description: errorMessage(error) });
    }
  }

  const list = cases.data ?? [];
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h3 className="text-sm text-neutral-300">{t("developer.cases")}</h3>
          <p className="text-xs text-neutral-500">{t("developer.casesHint")}</p>
        </div>
        <Button variant="neutral" className="text-xs" disabled={busy || list.length === 0} onClick={run}>
          {t("developer.casesRun")}
        </Button>
      </div>
      {cases.error && <p className="text-xs text-danger break-words">{cases.error}</p>}
      {list.length > 0 && (
        <ul className="max-h-80 overflow-auto divide-y divide-neutral-800/70 rounded-lg border border-neutral-800">
          {list.map((entry) => (
            <Row
              key={entry.name}
              entry={entry}
              result={results.find((result) => result.name === entry.name)}
              onAccept={(result) =>
                change(() => saveTestCase(name, { ...entry, expected: result.actual }))
              }
              onDelete={() => change(() => deleteTestCase(name, entry.name))}
            />
          ))}
        </ul>
      )}
    </div>
  );
}
