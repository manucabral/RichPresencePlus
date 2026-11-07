import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { importManualPreset } from "../../platform/api";
import Accordion from "../../shared/components/Accordion";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useT } from "../../shared/i18n";
import type { ManualPreset } from "../../shared/types";

/** Largest file read; the app refuses anything above 16 KB anyway. */
const MAX_FILE = 16 * 1024;

/**
 * Reads a shared custom status, pasted or from a file, into the editor.
 * Nothing is saved here: the user reviews it, application id included, and
 * saves it like one typed by hand.
 */
export default function ImportCard({ onLoaded }: { onLoaded: (preset: ManualPreset) => void }) {
  const { t } = useT();
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);

  async function load(source: string) {
    setBusy(true);
    try {
      onLoaded(await importManualPreset(source));
      setText("");
      toast(t("manual.imported"));
    } catch (error) {
      toast(t("manual.importFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  async function pick(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    if (file.size > MAX_FILE) {
      toast(t("manual.importFailed"), { description: t("manual.fileTooLarge") });
      return;
    }
    await load(await file.text());
  }

  return (
    <Accordion title={t("manual.import")}>
      <div className="space-y-2">
        <p className="text-xs text-neutral-500">{t("manual.importHint")}</p>
        <textarea
          rows={4}
          value={text}
          placeholder={t("manual.importPlaceholder")}
          className="w-full rounded-md border border-neutral-800 bg-neutral-950 px-3 py-2 font-mono text-xs text-neutral-200"
          onInput={(event) => setText((event.target as HTMLTextAreaElement).value)}
        />
        <div className="flex items-center gap-2">
          <Button variant="primary" className="text-xs" loading={busy} disabled={!text.trim()} onClick={() => load(text)}>
            {t("manual.importLoad")}
          </Button>
          <label className="cursor-pointer rounded-md border border-neutral-800 px-3 py-1.5 text-[13px] text-neutral-200 hover:bg-white/[0.04]">
            {t("manual.importFile")}
            <input type="file" accept=".json,application/json" className="sr-only" onChange={pick} />
          </label>
        </div>
      </div>
    </Accordion>
  );
}
