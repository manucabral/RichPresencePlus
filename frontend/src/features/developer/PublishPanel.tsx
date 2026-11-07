import type { ComponentChildren } from "preact";
import { useState } from "preact/hooks";
import { openLink, preparePresence, reviewPresence } from "../../platform/api";
import Button from "../../shared/components/Button";
import Linked from "../../shared/components/Linked";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { toast } from "../../shared/toast";
import { allPassed, sizeText } from "./workbench";

interface StepProps {
  number: number;
  title: string;
  hint: string;
  children?: ComponentChildren;
}

function Step({ number, title, hint, children }: StepProps) {
  return (
    <li className="flex gap-3">
      <span
        aria-hidden="true"
        className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-neutral-800 text-[11px] text-neutral-300"
      >
        {number}
      </span>
      <div className="min-w-0 flex-1 space-y-2">
        <div>
          <p className="text-sm text-neutral-200">{title}</p>
          <p className="text-xs text-neutral-500">{hint}</p>
        </div>
        {children}
      </div>
    </li>
  );
}

/** Takes a finished presence to the catalog, by a pull request that people review. */
export default function PublishPanel({ name }: { name: string }) {
  const { t } = useT();
  const review = useLoaded(() => reviewPresence(name));
  const [ready, setReady] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const checks = review.data ?? [];

  async function prepare() {
    setBusy(true);
    try {
      const copy = await preparePresence(name);
      setReady(t("developer.prepared", { files: copy.files, size: sizeText(copy.size) }));
    } catch (error) {
      setReady(null);
      toast(t("developer.prepareFailed"), { description: errorMessage(error) });
      review.reload();
    } finally {
      setBusy(false);
    }
  }

  async function upload() {
    try {
      await openLink("upload");
    } catch (error) {
      toast(t("legal.failed"), { description: errorMessage(error) });
    }
  }

  return (
    <section className="space-y-4 rounded-lg border border-neutral-800 bg-neutral-900/40 p-4">
      <div>
        <p className="text-sm text-neutral-200">{t("developer.publishTitle")}</p>
        <p className="text-xs text-neutral-500">
          <Linked text={t("developer.publishHint")} />
        </p>
      </div>
      <ol className="space-y-4">
        <Step number={1} title={t("developer.stepCheck")} hint={t("developer.stepCheckHint")}>
          {review.error && <p className="text-xs text-danger break-words">{review.error}</p>}
          <ul className="space-y-1 text-xs">
            {checks.map((check) => (
              <li key={check.rule} className="flex gap-2">
                <span aria-hidden="true" className={check.passed ? "text-accent-soft" : "text-danger"}>
                  {check.passed ? "✓" : "✕"}
                </span>
                <span className="min-w-0 break-words">
                  <span className={check.passed ? "text-neutral-400" : "text-neutral-200"}>
                    {t(`developer.rule.${check.rule}`)}
                  </span>
                  <span className="sr-only">
                    {" "}
                    {check.passed ? t("developer.rulePassed") : t("developer.ruleFailed")}
                  </span>
                  {check.detail && <span className="block text-danger">{check.detail}</span>}
                </span>
              </li>
            ))}
          </ul>
          <Button variant="ghost" className="text-xs" onClick={review.reload}>
            {t("developer.checkAgain")}
          </Button>
        </Step>
        <Step number={2} title={t("developer.stepPrepare")} hint={t("developer.stepPrepareHint")}>
          <Button variant="neutral" className="text-xs" loading={busy} disabled={!allPassed(checks)} onClick={prepare}>
            {t("developer.prepare")}
          </Button>
          {ready && <p className="text-xs text-accent-soft">{ready}</p>}
        </Step>
        <Step number={3} title={t("developer.stepUpload")} hint={t("developer.stepUploadHint")}>
          <Button variant="neutral" className="text-xs" disabled={ready === null} onClick={upload}>
            {t("developer.upload")}
          </Button>
        </Step>
        <Step number={4} title={t("developer.stepReview")} hint={t("developer.stepReviewHint")} />
      </ol>
    </section>
  );
}
