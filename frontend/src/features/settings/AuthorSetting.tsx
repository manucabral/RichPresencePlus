import { useEffect, useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { getAuthor, setAuthor } from "../../platform/api";
import Button from "../../shared/components/Button";
import { SettingRow, fieldClass } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";

/** The author written into the presences the user creates or copies. */
export default function AuthorSetting() {
  const { t } = useT();
  const author = useLoaded(getAuthor);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const saved = author.data?.saved ?? "";
  const discord = author.data?.discord ?? null;

  // Show what is saved once it arrives; typing is not overwritten after that.
  useEffect(() => setText(saved), [saved]);

  async function save() {
    setBusy(true);
    try {
      await setAuthor(text);
      author.reload();
      toast(t("settings.authorSaved"));
    } catch (error) {
      toast(t("settings.saveFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <SettingRow
      stacked
      title={t("settings.author")}
      hint={discord ? t("settings.authorHintDiscord", { name: discord }) : t("settings.authorHint")}
    >
      <div className="flex gap-2">
        <input
          className={fieldClass}
          value={text}
          maxLength={64}
          placeholder={discord ?? t("settings.authorPlaceholder")}
          aria-label={t("settings.author")}
          onInput={(event) => setText((event.target as HTMLInputElement).value)}
        />
        <Button variant="primary" loading={busy} disabled={text.trim() === saved} onClick={save}>
          {t("settings.authorSave")}
        </Button>
      </div>
    </SettingRow>
  );
}
