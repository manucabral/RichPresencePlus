import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { getDiagnostics } from "../../platform/api";
import Button from "../../shared/components/Button";
import { browserName } from "../../shared/connection";
import { Rows } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useT } from "../../shared/i18n";
import type { Diagnostics, Resources } from "../../shared/types";

function megabytes(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function Line({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-4 py-1.5 text-[13px]">
      <span className="text-neutral-500">{label}</span>
      <span className="min-w-0 truncate text-right text-neutral-200" title={value}>
        {value}
      </span>
    </div>
  );
}

/** Diagnostics, read only when asked: the reading measures processes. */
export default function DiagnosticsSetting() {
  const { t, tn } = useT();
  const [data, setData] = useState<Diagnostics | null>(null);
  const [busy, setBusy] = useState(false);

  async function read() {
    setBusy(true);
    try {
      setData(await getDiagnostics());
    } catch (error) {
      toast(t("diagnostics.failed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  async function copy() {
    if (!data) return;
    try {
      await navigator.clipboard.writeText(data.report);
      toast(t("diagnostics.copied"));
    } catch {
      toast(t("browser.copyFailed"));
    }
  }

  const usage = (part: Resources) =>
    t("diagnostics.usage", { memory: megabytes(part.memory_bytes), cpu: part.cpu_seconds.toFixed(1) });
  const yes = (value: boolean) => (value ? t("diagnostics.yes") : t("diagnostics.no"));

  return (
    <Rows>
      <div className="flex items-center justify-between gap-3 px-4 py-3">
        <p className="text-[13px] text-neutral-400">{data ? t("diagnostics.version") + " " + data.version : t("diagnostics.notRead")}</p>
        <div className="flex shrink-0 gap-2">
          <Button variant="ghost" className="text-xs" onClick={read} loading={busy}>
            {data ? t("diagnostics.again") : t("diagnostics.read")}
          </Button>
          <Button variant="primary" className="text-xs" onClick={copy} disabled={!data}>
            {t("diagnostics.copy")}
          </Button>
        </div>
      </div>

      {data && (
        <>
          <div className="px-4 py-2">
            {data.resources ? (
              <>
                <Line label={t("diagnostics.app")} value={usage(data.resources.app)} />
                <Line
                  label={tn("diagnostics.webview", data.resources.webview.processes)}
                  value={usage(data.resources.webview)}
                />
              </>
            ) : (
              <Line label={t("diagnostics.resources")} value={t("diagnostics.unavailable")} />
            )}
            <Line label={t("diagnostics.discordRunning")} value={yes(data.discord_running)} />
            <Line label={t("diagnostics.discordConnected")} value={yes(data.discord_connected)} />
            {data.delivery_error && <Line label={t("diagnostics.discordError")} value={data.delivery_error} />}
            <Line
              label={t("diagnostics.extensions")}
              value={
                data.extensions.length === 0
                  ? t("dashboard.noExtension")
                  : data.extensions.map((s) => `${browserName(s.browser, t)} ${s.version}`.trim()).join(", ")
              }
            />
          </div>
          <ul className="px-4 py-2">
            {data.presences.map((presence) => (
              <li key={presence.name} className="py-1.5 text-[13px]">
                <div className="flex justify-between gap-4">
                  <span className="text-neutral-200">
                    {presence.name} <span className="text-neutral-600">{presence.version}</span>
                  </span>
                  <span className={presence.error ? "text-danger" : "text-neutral-500"}>
                    {!presence.enabled
                      ? t("diagnostics.off")
                      : presence.error
                        ? t("diagnostics.error")
                        : presence.running
                          ? t("diagnostics.running")
                          : t("diagnostics.stopped")}
                  </span>
                </div>
                {presence.error && <p className="break-words text-xs text-danger/80">{presence.error}</p>}
              </li>
            ))}
          </ul>
          <p className="px-4 py-2.5 text-xs text-neutral-600">{t("diagnostics.private")}</p>
        </>
      )}
    </Rows>
  );
}
