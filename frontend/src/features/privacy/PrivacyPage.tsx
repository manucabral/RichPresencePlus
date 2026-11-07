import { useEffect, useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { getPreferences, savePreferences } from "../../platform/api";
import Button from "../../shared/components/Button";
import PageTitle from "../../shared/components/PageTitle";
import Switch from "../../shared/components/Switch";
import { Panel, Section, SettingRow, areaClass, fieldClass, selectClass, pageWidth } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { clock, edited, lines, minutesOf } from "./form";
import ProfilesCard from "./ProfilesCard";
import { useT } from "../../shared/i18n";
import type { GlobalPreferences } from "../../shared/types";

// Choices for hiding the activity when idle; the backend accepts 5 to 120.
const IDLE_CHOICES = [5, 10, 15, 30, 60, 120];
// Minutes paused media stays; the backend accepts 0 to 60.
const PAUSED_CHOICES = [1, 5, 15, 30, 60];
// The night, when the switch is first turned on.
const NIGHT = { from: 22 * 60, to: 7 * 60 };

export default function PrivacyPage() {
  const { t, tn } = useT();
  const preferences = useLoaded(getPreferences);
  const [form, setForm] = useState<GlobalPreferences | null>(null);
  const [domains, setDomains] = useState("");
  const [applications, setApplications] = useState("");
  const [quiet, setQuiet] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const global = preferences.data?.global;
    if (!global) return;
    setForm(global);
    setDomains(global.excluded_domains.join("\n"));
    setApplications(global.excluded_applications.join("\n"));
    setQuiet(global.quiet_programs.join("\n"));
  }, [preferences.data]);

  if (!form) return <p className="text-sm text-neutral-500">{preferences.error ?? t("app.loading")}</p>;

  const changed = preferences.data ? edited(preferences.data.global, form, domains, applications, quiet) : false;
  const hours = form.quiet_hours;
  // An empty box leaves the hour as it was; the time box itself says it is empty.
  const setHour = (end: "from" | "to", text: string) => {
    const minutes = minutesOf(text);
    if (hours && minutes !== null) setForm({ ...form, quiet_hours: { ...hours, [end]: minutes } });
  };

  async function save() {
    if (!form || !preferences.data) return;
    setSaving(true);
    try {
      await savePreferences({
        ...preferences.data,
        global: { ...form, excluded_domains: lines(domains), excluded_applications: lines(applications), quiet_programs: lines(quiet) },
      });
      toast(t("privacy.saved"));
      preferences.reload();
    } catch (error) {
      toast(t("privacy.saveFailed"), { description: errorMessage(error) });
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className={pageWidth}>
      <PageTitle title={t("privacy.title")} subtitle={t("privacy.subtitle")} />

      <Section title={t("privacy.publishing")}>
        <Panel>
          <SettingRow title={t("privacy.pause")} hint={t("privacy.pauseHint")}>
            <Switch checked={form.paused} onChange={(paused) => setForm({ ...form, paused })} label={t("privacy.pause")} />
          </SettingRow>
          <SettingRow title={t("privacy.recent")} hint={t("privacy.recentHint")}>
            <Switch
              checked={form.prefer_recent}
              onChange={(prefer_recent) => setForm({ ...form, prefer_recent })}
              label={t("privacy.recent")}
            />
          </SettingRow>
          <SettingRow title={t("privacy.delay")} hint={t("privacy.delayHint")}>
            <input
              type="number"
              min={0}
              max={30}
              value={form.switch_delay_seconds}
              aria-label={t("privacy.delay")}
              className={`${fieldClass} w-20 text-right`}
              onChange={(event) =>
                setForm({ ...form, switch_delay_seconds: Number((event.target as HTMLInputElement).value) })
              }
            />
          </SettingRow>
          <SettingRow title={t("privacy.idle")} hint={t("privacy.idleHint")}>
            <select
              value={form.idle_minutes === null ? "" : String(form.idle_minutes)}
              aria-label={t("privacy.idle")}
              className={selectClass}
              onChange={(event) => {
                const value = (event.target as HTMLSelectElement).value;
                setForm({ ...form, idle_minutes: value === "" ? null : Number(value) });
              }}
            >
              <option value="">{t("privacy.idleOff")}</option>
              {IDLE_CHOICES.map((minutes) => (
                <option key={minutes} value={String(minutes)}>
                  {tn("privacy.idleAfter", minutes)}
                </option>
              ))}
            </select>
          </SettingRow>
          <SettingRow title={t("privacy.paused")} hint={t("privacy.pausedHint")}>
            <select
              value={String(form.paused_minutes)}
              aria-label={t("privacy.paused")}
              className={selectClass}
              onChange={(event) =>
                setForm({ ...form, paused_minutes: Number((event.target as HTMLSelectElement).value) })
              }
            >
              <option value="0">{t("privacy.pausedHide")}</option>
              {PAUSED_CHOICES.map((minutes) => (
                <option key={minutes} value={String(minutes)}>
                  {tn("privacy.pausedKeep", minutes)}
                </option>
              ))}
            </select>
          </SettingRow>
        </Panel>
      </Section>

      <Section title={t("privacy.detail")}>
        <Panel>
          {(
            [
              ["full", t("privacy.detailFull")],
              ["service", t("privacy.detailService")],
            ] as const
          ).map(([value, label]) => (
            <label key={value} className="flex cursor-pointer items-center gap-3 px-4 py-3 text-[13px] text-neutral-200">
              <input
                type="radio"
                name="privacy"
                checked={form.privacy === value}
                onChange={() => setForm({ ...form, privacy: value })}
              />
              {label}
            </label>
          ))}
        </Panel>
      </Section>

      <Section title={t("privacy.exclusions")}>
        <Panel>
          <SettingRow stacked title={t("privacy.sites")} hint={t("privacy.sitesHint")}>
            <textarea
              rows={4}
              aria-label={t("privacy.sites")}
              className={`${areaClass} font-mono text-xs`}
              value={domains}
              onInput={(event) => setDomains((event.target as HTMLTextAreaElement).value)}
            />
          </SettingRow>
          <SettingRow stacked title={t("privacy.programs")} hint={t("privacy.programsHint")}>
            <textarea
              rows={3}
              aria-label={t("privacy.programs")}
              className={`${areaClass} font-mono text-xs`}
              value={applications}
              onInput={(event) => setApplications((event.target as HTMLTextAreaElement).value)}
            />
          </SettingRow>
        </Panel>
      </Section>

      <Section title={t("privacy.auto")} hint={t("privacy.autoHint")}>
        <Panel>
          <SettingRow stacked title={t("privacy.quietPrograms")} hint={t("privacy.quietProgramsHint")}>
            <textarea
              rows={3}
              aria-label={t("privacy.quietPrograms")}
              className={`${areaClass} font-mono text-xs`}
              value={quiet}
              onInput={(event) => setQuiet((event.target as HTMLTextAreaElement).value)}
            />
          </SettingRow>
          <SettingRow title={t("privacy.quietHours")} hint={t("privacy.quietHoursHint")}>
            <Switch
              checked={hours !== null}
              onChange={(on) => setForm({ ...form, quiet_hours: on ? NIGHT : null })}
              label={t("privacy.quietHours")}
            />
          </SettingRow>
          {hours && (
            <SettingRow title={t("privacy.quietFrom")}>
              <div className="flex items-center gap-2">
                <input
                  type="time"
                  aria-label={t("privacy.quietFrom")}
                  className={fieldClass}
                  value={clock(hours.from)}
                  onChange={(event) => setHour("from", (event.target as HTMLInputElement).value)}
                />
                <span className="text-xs text-neutral-500">{t("privacy.quietTo")}</span>
                <input
                  type="time"
                  aria-label={t("privacy.quietTo")}
                  className={fieldClass}
                  value={clock(hours.to)}
                  onChange={(event) => setHour("to", (event.target as HTMLInputElement).value)}
                />
              </div>
            </SettingRow>
          )}
        </Panel>
      </Section>

      {changed && (
        <div className="sticky bottom-0 z-10 -mx-1 flex items-center gap-3 border-t border-neutral-800 bg-neutral-950 px-1 py-3">
          <Button variant="primary" onClick={save} loading={saving}>
            {t("privacy.save")}
          </Button>
          <p className="text-xs text-neutral-500">{t("privacy.note")}</p>
        </div>
      )}

      <Section title={t("profiles.title")} hint={t("profiles.hint")}>
        <ProfilesCard onApplied={preferences.reload} />
      </Section>
    </div>
  );
}
