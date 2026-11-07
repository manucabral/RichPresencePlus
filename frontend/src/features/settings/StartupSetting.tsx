import { toast } from "../../shared/toast";
import { getStartup, saveStartup } from "../../platform/api";
import Switch from "../../shared/components/Switch";
import { Panel, SettingRow } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import type { StartupView } from "../../shared/types";

/** Starting with Windows and what the window does when minimized. */
export default function StartupSetting() {
  const { t } = useT();
  const startup = useLoaded(getStartup);
  const current = startup.data;
  if (!current) return null;

  async function change(next: Partial<StartupView>) {
    const merged = { ...current!, ...next };
    try {
      await saveStartup(merged.enabled, merged.hidden, merged.minimize_to_tray);
    } catch (error) {
      toast(t("settings.saveFailed"), { description: errorMessage(error) });
    }
    startup.reload();
  }

  return (
    <Panel>
      <SettingRow title={t("startup.enabled")} hint={t("startup.enabledHint")}>
        <Switch
          checked={current.enabled}
          label={t("startup.enabled")}
          onChange={(enabled) => change({ enabled, hidden: enabled ? true : current.hidden })}
        />
      </SettingRow>
      <SettingRow title={t("startup.hidden")} hint={t("startup.hiddenHint")}>
        <Switch
          checked={current.enabled && current.hidden}
          disabled={!current.enabled}
          label={t("startup.hidden")}
          onChange={(hidden) => change({ hidden })}
        />
      </SettingRow>
      <SettingRow title={t("startup.minimize")} hint={t("startup.minimizeHint")}>
        <Switch
          checked={current.minimize_to_tray}
          label={t("startup.minimize")}
          onChange={(minimize_to_tray) => change({ minimize_to_tray })}
        />
      </SettingRow>
    </Panel>
  );
}
