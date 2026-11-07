import { useEffect, useRef, useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { getLanguage, getLogLevel, getLogs, setLogLevel } from "../../platform/api";
import Accordion from "../../shared/components/Accordion";
import Button from "../../shared/components/Button";
import PageTitle from "../../shared/components/PageTitle";
import { Panel, Rows, Section, SettingRow, selectClass, pageWidth } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { available, chooseLanguage, current, useT } from "../../shared/i18n";
import AuthorSetting from "./AuthorSetting";
import DiagnosticsSetting from "./DiagnosticsSetting";
import DiscordClientSetting from "./DiscordClientSetting";
import LocalApiSetting from "./LocalApiSetting";
import LegalSetting from "./LegalSetting";
import ShortcutSetting from "./ShortcutSetting";
import StartupSetting from "./StartupSetting";

const LEVELS = ["error", "warn", "info", "debug"] as const;

/** Names a language in its own words, as the system knows it. */
function languageName(tag: string): string {
  try {
    return new Intl.DisplayNames([tag], { type: "language" }).of(tag) ?? tag;
  } catch {
    return tag;
  }
}

function LanguageSetting() {
  const { t } = useT();
  const saved = useLoaded(getLanguage);

  async function change(setting: string) {
    try {
      await chooseLanguage(setting);
      saved.reload();
    } catch (error) {
      toast(t("settings.saveFailed"), { description: errorMessage(error) });
    }
  }

  return (
    <SettingRow title={t("language.label")}>
      <select
        value={saved.data?.setting ?? "auto"}
        aria-label={t("language.label")}
        className={selectClass}
        onChange={(event) => change((event.target as HTMLSelectElement).value)}
      >
        <option value="auto">{`${t("language.auto")} (${languageName(saved.data?.system ?? current())})`}</option>
        {available.map((tag) => (
          <option key={tag} value={tag}>
            {languageName(tag)}
          </option>
        ))}
      </select>
    </SettingRow>
  );
}

function LogSetting() {
  const { t } = useT();
  const level = useLoaded(getLogLevel);
  const logs = useLoaded(getLogs);
  const [copied, setCopied] = useState(false);
  const box = useRef<HTMLPreElement>(null);

  // The newest lines are at the end; show them first.
  useEffect(() => {
    if (box.current) box.current.scrollTop = box.current.scrollHeight;
  }, [logs.data]);

  async function change(value: string) {
    try {
      await setLogLevel(value);
      level.reload();
    } catch (error) {
      toast(t("settings.saveFailed"), { description: errorMessage(error) });
    }
  }

  async function copyFolder() {
    if (!logs.data) return;
    try {
      await navigator.clipboard.writeText(logs.data.folder);
      setCopied(true);
    } catch {
      toast(t("browser.copyFailed"));
    }
  }

  return (
    <Rows>
      <SettingRow title={t("settings.logLevel")} hint={t("settings.logLevelHint")}>
        <select
          value={level.data ?? "info"}
          aria-label={t("settings.logLevel")}
          className={selectClass}
          onChange={(event) => change((event.target as HTMLSelectElement).value)}
        >
          {LEVELS.map((name) => (
            <option key={name} value={name}>
              {t(`settings.level.${name}`)}
            </option>
          ))}
        </select>
      </SettingRow>
      <div className="space-y-3 px-4 py-3">
        <div className="flex items-center justify-between gap-3">
          <p className="truncate text-xs text-neutral-500" title={logs.data?.folder}>
            {logs.data?.folder ?? logs.error ?? t("app.loading")}
          </p>
          <div className="flex shrink-0 gap-2">
            <Button variant="ghost" className="text-xs" onClick={copyFolder} disabled={!logs.data}>
              {copied ? t("settings.copied") : t("settings.copyFolder")}
            </Button>
            <Button variant="ghost" className="text-xs" onClick={logs.reload}>
              {t("settings.refresh")}
            </Button>
          </div>
        </div>
        <pre
          ref={box}
          className="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-md border border-neutral-800 bg-neutral-950 p-3 font-mono text-[11px] leading-relaxed text-neutral-400"
        >
          {logs.data && logs.data.lines.length > 0 ? logs.data.lines.join("\n") : t("settings.noLogs")}
        </pre>
      </div>
    </Rows>
  );
}

export default function SettingsPage() {
  const { t } = useT();
  return (
    <div className={pageWidth}>
      <PageTitle title={t("settings.title")} subtitle={t("settings.subtitle")} />
      <Section title={t("settings.general")}>
        <Panel>
          <LanguageSetting />
          <DiscordClientSetting />
          <ShortcutSetting />
          <AuthorSetting />
          <LocalApiSetting />
        </Panel>
      </Section>
      <Section title={t("startup.title")}>
        <StartupSetting />
      </Section>
      <Section title={t("legal.title")} hint={t("legal.hint")}>
        <Panel>
          <LegalSetting />
        </Panel>
      </Section>
      <Section title={t("settings.support")}>
        <div className="space-y-2">
          <Accordion flush title={t("diagnostics.title")}>
            <p className="px-4 pt-3 text-xs text-neutral-500">{t("diagnostics.hint")}</p>
            <DiagnosticsSetting />
          </Accordion>
          <Accordion flush title={t("settings.logs")}>
            <LogSetting />
          </Accordion>
        </div>
      </Section>
    </div>
  );
}
