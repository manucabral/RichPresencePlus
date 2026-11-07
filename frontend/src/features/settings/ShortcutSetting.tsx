import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { getPauseShortcut, setPauseShortcut } from "../../platform/api";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { shortcutOf } from "./shortcut";
import { SettingRow } from "../../shared/components/ui";

/** A shortcut that pauses or resumes publishing from any program; off until set. */
export default function ShortcutSetting() {
  const { t } = useT();
  const saved = useLoaded(getPauseShortcut);
  const [typed, setTyped] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function save(shortcut: string | null) {
    setBusy(true);
    try {
      await setPauseShortcut(shortcut);
      setTyped(null);
      saved.reload();
      toast(shortcut ? t("settings.shortcutSaved", { shortcut }) : t("settings.shortcutOff"));
    } catch (error) {
      toast(t("settings.shortcutFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  function record(event: KeyboardEvent) {
    // Tab still moves the focus; every other key is read, not typed.
    if (event.key === "Tab") return;
    event.preventDefault();
    const shortcut = shortcutOf(event);
    if (shortcut) setTyped(shortcut);
  }

  const shown = typed ?? saved.data ?? "";

  return (
    <SettingRow title={t("settings.shortcut")} hint={t("settings.shortcutHint")}>
      <div className="flex items-center gap-2">
        <input
          readOnly
          value={shown}
          placeholder={t("settings.shortcutPlaceholder")}
          aria-label={t("settings.shortcut")}
          onKeyDown={record}
          className="w-36 rounded-md border border-neutral-800 bg-neutral-950 px-2 py-1 text-center font-mono text-xs text-neutral-100 placeholder:font-sans placeholder:text-neutral-600"
        />
        {typed && typed !== saved.data && (
          <Button variant="primary" className="text-xs" loading={busy} onClick={() => save(typed)}>
            {t("settings.shortcutSave")}
          </Button>
        )}
        {saved.data && !typed && (
          <Button variant="ghost" className="text-xs" disabled={busy} onClick={() => save(null)}>
            {t("settings.shortcutRemove")}
          </Button>
        )}
      </div>
    </SettingRow>
  );
}
