import { useEffect, useRef, useState } from "preact/hooks";
import { toast } from "../toast";
import { getStartup, saveStartup } from "../../platform/api";
import { errorMessage } from "../errors";
import { useLoaded } from "../hooks";
import { useT } from "../i18n";
import Button from "./Button";

/**
 * Asks once how the application should start, unless the installer already asked.
 */
export default function FirstRun() {
  const { t } = useT();
  const startup = useLoaded(getStartup);
  const dialog = useRef<HTMLDialogElement>(null);
  const [enabled, setEnabled] = useState(true);
  const [hidden, setHidden] = useState(true);
  const [minimize, setMinimize] = useState(true);
  const [busy, setBusy] = useState(false);
  const open = startup.data !== null && !startup.data.asked;

  useEffect(() => {
    if (open && dialog.current && !dialog.current.open) dialog.current.showModal();
  }, [open]);

  async function answer(start: boolean) {
    setBusy(true);
    try {
      await saveStartup(start && enabled, start && hidden, start ? minimize : true);
      dialog.current?.close();
      startup.reload();
    } catch (error) {
      toast(t("settings.saveFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  if (!open) return null;
  const box = "flex items-start gap-2 text-sm text-neutral-200";
  return (
    <dialog
      ref={dialog}
      className="m-auto w-96 max-w-[90vw] rounded-lg border border-neutral-800 bg-neutral-900 p-5 text-neutral-100 shadow-2xl"
      aria-label={t("startup.firstTitle")}
      // Escape would leave the question unanswered; the buttons answer it.
      onCancel={(event) => event.preventDefault()}
    >
      <h2 className="text-base font-semibold">{t("startup.firstTitle")}</h2>
      <p className="mt-1 text-xs text-neutral-500">{t("startup.firstHint")}</p>
      <div className="mt-4 space-y-3">
        <label className={box}>
          <input type="checkbox" checked={enabled} onChange={() => setEnabled(!enabled)} />
          {t("startup.enabled")}
        </label>
        <label className={`${box} ${enabled ? "" : "opacity-50"}`}>
          <input type="checkbox" checked={hidden} disabled={!enabled} onChange={() => setHidden(!hidden)} />
          {t("startup.hidden")}
        </label>
        <label className={box}>
          <input type="checkbox" checked={minimize} onChange={() => setMinimize(!minimize)} />
          {t("startup.minimize")}
        </label>
      </div>
      <div className="mt-5 flex justify-end gap-2">
        <Button variant="ghost" disabled={busy} onClick={() => answer(false)}>
          {t("startup.notNow")}
        </Button>
        <Button variant="primary" loading={busy} onClick={() => answer(true)}>
          {t("startup.save")}
        </Button>
      </div>
    </dialog>
  );
}
