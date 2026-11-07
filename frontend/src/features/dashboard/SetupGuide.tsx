import { useState } from "preact/hooks";
import { Link } from "../../shared/router";
import Button from "../../shared/components/Button";
import { Panel } from "../../shared/components/ui";
import { useT } from "../../shared/i18n";
import { nextStep, setupSteps, type SetupInput } from "./setup";

const STORED = "rpp.setupClosed";

function wasClosed(): boolean {
  try {
    return localStorage.getItem(STORED) === "1";
  } catch {
    return false; // Without storage the guide simply shows again.
  }
}

function remember() {
  try {
    localStorage.setItem(STORED, "1");
  } catch {
    // The choice lasts until the window closes.
  }
}

/** Shows what is left until the first activity, and goes away once it is done. */
export default function SetupGuide(input: SetupInput) {
  const { t } = useT();
  const [closed, setClosed] = useState(wasClosed);
  const steps = setupSteps(input);
  const next = nextStep(steps);
  if (closed) return null;
  if (next === null) {
    remember();
    return null;
  }
  const close = () => {
    remember();
    setClosed(true);
  };
  const done = steps.filter((step) => step.done).length;

  return (
    <section className="mb-6" aria-label={t("setup.title")}>
      <div className="mb-3 flex items-center justify-between gap-3">
        <h2 className="text-[13px] font-medium text-neutral-400">
          {t("setup.title")} <span className="text-neutral-600">{t("setup.progress", { done, total: steps.length })}</span>
        </h2>
        <Button variant="ghost" className="text-xs" onClick={close}>
          {t("setup.close")}
        </Button>
      </div>
      <Panel>
        {steps.map((step) => (
          <div key={step.id} className="flex items-center gap-3 px-3 py-2.5">
            <span
              aria-hidden="true"
              className={`flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-[11px] font-semibold ${
                step.done ? "bg-accent text-accent-ink" : "border border-neutral-700 text-neutral-500"
              }`}
            >
              {step.done ? "✓" : ""}
            </span>
            <div className="min-w-0 flex-1">
              <p className={`text-[13px] ${step.done ? "text-neutral-500" : "text-neutral-100"}`}>{t(`setup.${step.id}`)}</p>
              {step.id === next && <p className="text-xs text-neutral-500">{t(`setup.${step.id}Hint`)}</p>}
            </div>
            {step.id === next && step.link && (
              <Link to={step.link} className="rounded-md border border-neutral-800 px-2.5 py-1 text-xs text-neutral-200 hover:bg-white/[0.04]">
                {t("setup.open")}
              </Link>
            )}
          </div>
        ))}
      </Panel>
    </section>
  );
}
