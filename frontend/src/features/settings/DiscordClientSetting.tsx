import { toast } from "../../shared/toast";
import { getDiscordClient, setDiscordClient } from "../../platform/api";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { SettingRow, selectClass } from "../../shared/components/ui";
import type { DiscordClient } from "../../shared/types";

const CHOICES: DiscordClient[] = ["auto", "stable", "ptb", "canary"];

/** Which Discord client receives the activity when several are open. */
export default function DiscordClientSetting() {
  const { t } = useT();
  const saved = useLoaded(getDiscordClient);

  async function change(choice: DiscordClient) {
    try {
      await setDiscordClient(choice);
      saved.reload();
    } catch (error) {
      toast(t("settings.discordClientFailed"), { description: errorMessage(error) });
    }
  }

  return (
    <SettingRow title={t("settings.discordClient")} hint={t("settings.discordClientHint")}>
      <select
        value={saved.data ?? "auto"}
        disabled={!saved.data}
        aria-label={t("settings.discordClient")}
        className={selectClass}
        onChange={(event) => change((event.target as HTMLSelectElement).value as DiscordClient)}
      >
        {CHOICES.map((choice) => (
          <option key={choice} value={choice}>
            {t(`settings.discordClient.${choice}`)}
          </option>
        ))}
      </select>
    </SettingRow>
  );
}
