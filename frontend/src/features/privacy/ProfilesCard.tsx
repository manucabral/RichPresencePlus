import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { applyProfile, deleteProfile, getProfiles, saveProfile } from "../../platform/api";
import Button from "../../shared/components/Button";
import { Panel, fieldClass } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";

interface Props {
  /** Called after a profile is applied, so the page reads the new rules. */
  onApplied: () => void;
}

/** Saves the current rules under a name and switches between saved sets. */
export default function ProfilesCard({ onApplied }: Props) {
  const { t } = useT();
  const profiles = useLoaded(getProfiles);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);

  async function run(action: () => Promise<void>, failure: string, after?: () => void) {
    setBusy(true);
    try {
      await action();
      profiles.reload();
      after?.();
    } catch (error) {
      toast(failure, { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <Panel>
      <div className="flex gap-2 px-4 py-3">
        <input
          className={fieldClass}
          value={name}
          maxLength={64}
          placeholder={t("profiles.name")}
          aria-label={t("profiles.name")}
          onInput={(event) => setName((event.target as HTMLInputElement).value)}
        />
        <Button
          variant="primary"
          loading={busy}
          disabled={name.trim() === ""}
          onClick={() => run(() => saveProfile(name.trim()), t("profiles.saveFailed"), () => setName(""))}
        >
          {t("profiles.save")}
        </Button>
      </div>
      {profiles.data && profiles.data.length > 0 && (
        <ul className="divide-y divide-neutral-800/70">
          {profiles.data.map((profile) => (
            <li key={profile.name} className="flex items-center justify-between gap-3 px-4 py-2">
              <span className="truncate text-[13px] text-neutral-200">{profile.name}</span>
              <span className="flex shrink-0 gap-1">
                <Button
                  variant="ghost"
                  className="text-xs"
                  disabled={busy}
                  onClick={() => run(() => applyProfile(profile.name), t("profiles.applyFailed"), onApplied)}
                >
                  {t("profiles.apply")}
                </Button>
                <Button
                  variant="ghost"
                  className="text-xs"
                  disabled={busy}
                  onClick={() => run(() => deleteProfile(profile.name), t("profiles.deleteFailed"))}
                >
                  {t("profiles.delete")}
                </Button>
              </span>
            </li>
          ))}
        </ul>
      )}
    </Panel>
  );
}
