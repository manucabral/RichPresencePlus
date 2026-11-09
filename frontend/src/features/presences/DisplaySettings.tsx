import { toast } from "../../shared/toast";
import { getPreferences, savePreferences, setPresenceOption } from "../../platform/api";
import Switch from "../../shared/components/Switch";
import { Panel, Section, SettingRow, fieldClass, selectClass } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { shownOptions } from "../../shared/presence";
import type { PresencePreferences, PresenceView, StatusDisplay } from "../../shared/types";
import OptionControl from "./OptionControl";
import UserSites from "./UserSites";

const DEFAULTS: PresencePreferences = {
  enabled: true,
  priority: null,
  show_title: true,
  show_buttons: true,
  show_timestamps: true,
  status_display: null,
  hide_when_paused: null,
};

/** What Discord shows for a presence, and the presence's own options. */
export default function DisplaySettings({ presence, onSaved }: { presence: PresenceView; onSaved: () => void }) {
  const { t } = useT();
  const preferences = useLoaded(getPreferences);
  if (!preferences.data) return <p className="text-xs text-neutral-500">{preferences.error ?? t("app.loading")}</p>;
  const current = preferences.data.presences[presence.name] ?? DEFAULTS;

  async function saveOption(key: string, value: boolean | number | string) {
    try {
      await setPresenceOption(presence.name, key, value);
    } catch (error) {
      toast(t("presences.saveFailed"), { description: errorMessage(error) });
    }
    onSaved();
  }

  async function update(change: Partial<PresencePreferences>) {
    try {
      // Read again just before saving: another page may have changed other rules.
      const all = await getPreferences();
      all.presences[presence.name] = { ...(all.presences[presence.name] ?? DEFAULTS), ...change };
      await savePreferences(all);
      preferences.reload();
      onSaved();
    } catch (error) {
      toast(t("presences.saveFailed"), { description: errorMessage(error) });
    }
  }

  const toggle = (key: "show_title" | "show_buttons" | "show_timestamps", title: string) => (
    <SettingRow title={title}>
      <Switch checked={current[key]} label={title} onChange={(value) => update({ [key]: value })} />
    </SettingRow>
  );
  const options = shownOptions(presence.options);

  return (
    <>
      {presence.user_sites && <UserSites name={presence.name} sites={presence.user_sites} onSaved={onSaved} />}
      <Section title={t("presences.display")}>
        <Panel>
          {toggle("show_title", t("presences.showTitles"))}
          {toggle("show_buttons", t("presences.showButtons"))}
          {toggle("show_timestamps", t("presences.showTime"))}
          <SettingRow title={t("presences.hideWhenPaused")}>
            <Switch
              // Unset follows the developer's choice in the manifest.
              checked={current.hide_when_paused ?? presence.hide_when_paused}
              label={t("presences.hideWhenPaused")}
              onChange={(value) => update({ hide_when_paused: value })}
            />
          </SettingRow>
          <SettingRow title={t("presences.memberList")}>
            <select
              value={current.status_display ?? ""}
              aria-label={t("presences.memberList")}
              className={selectClass}
              onChange={(event) => {
                const value = (event.target as HTMLSelectElement).value;
                update({ status_display: value === "" ? null : (value as StatusDisplay) });
              }}
            >
              <option value="">{t("presences.memberListPresence")}</option>
              <option value="name">{t("presences.memberListName")}</option>
              <option value="details">{t("presences.memberListDetails")}</option>
              <option value="state">{t("presences.memberListState")}</option>
            </select>
          </SettingRow>
          <SettingRow title={t("presences.priority")}>
            <input
              type="number"
              min={0}
              max={100}
              value={current.priority ?? ""}
              aria-label={t("presences.priority")}
              className={`${fieldClass} w-20 text-right`}
              onChange={(event) => {
                const text = (event.target as HTMLInputElement).value;
                update({ priority: text === "" ? null : Number(text) });
              }}
            />
          </SettingRow>
        </Panel>
      </Section>
      {options.length > 0 && (
        <Section title={t("presences.ownOptions")}>
          <Panel>
            {options.map((option) => (
              <div key={option.key} className="px-4 py-3">
                <OptionControl option={option} onSave={(value) => saveOption(option.key, value)} />
                {option.help && <p className="mt-1 text-xs text-neutral-500">{option.help}</p>}
              </div>
            ))}
          </Panel>
        </Section>
      )}
    </>
  );
}
