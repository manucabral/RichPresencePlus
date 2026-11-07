import { toast } from "../../shared/toast";
import { watchPresence } from "../../platform/api";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useT } from "../../shared/i18n";
import Inspector from "./Inspector";
import type { Pick } from "./usePick";

const field = "rounded-md border border-neutral-800 bg-neutral-950 px-3 py-1.5 text-[13px] text-neutral-100 placeholder:text-neutral-600";

/** Watches one of the user's presences and shows what it reads, live. */
export default function LiveCard({ pick, onCaseSaved }: { pick: Pick; onCaseSaved: () => void }) {
  const { t } = useT();
  const { all, name, canWatch, watching, status, setPicked } = pick;

  async function toggle() {
    try {
      await watchPresence(watching ? null : name);
      status.reload();
    } catch (error) {
      toast(t("developer.watchFailed"), { description: errorMessage(error) });
    }
  }

  const reloaded = status.data?.last_reload;
  return (
    <section className="p-4 rounded-lg bg-neutral-900/40 border border-neutral-800 space-y-3">
      <div>
        <p className="text-sm text-neutral-200">{t("developer.live")}</p>
        <p className="text-xs text-neutral-500">{t("developer.liveHint")}</p>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <select
          value={name}
          disabled={watching !== null}
          className={field}
          aria-label={t("developer.pick")}
          onChange={(event) => setPicked((event.target as HTMLSelectElement).value)}
        >
          {all.map((presence) => (
            <option key={presence.name} value={presence.name}>
              {presence.name}
              {presence.editable ? "" : ` (${t("developer.readOnly")})`}
            </option>
          ))}
        </select>
        <Button
          variant={watching ? "warning" : "primary"}
          className="text-xs"
          disabled={name === "" || (!watching && !canWatch)}
          onClick={toggle}
        >
          {watching ? t("developer.stop") : t("developer.watch")}
        </Button>
      </div>
      {name !== "" && !canWatch && !watching && <p className="text-xs text-neutral-500">{t("developer.copyToWatch")}</p>}
      {watching && (
        <div className="text-xs text-neutral-500 space-y-0.5">
          <p className="truncate" title={status.data?.folder ?? ""}>
            {t("developer.watching", { folder: status.data?.folder ?? "" })}
          </p>
          {reloaded && <p>{t("developer.lastReload", { time: new Date(reloaded * 1000).toLocaleTimeString() })}</p>}
          {status.data?.error && <p className="text-danger break-words">{status.data.error}</p>}
        </div>
      )}
      {name !== "" && (
        <Inspector
          key={`${name}:${watching !== null}`}
          name={name}
          live={watching !== null}
          onCaseSaved={canWatch ? onCaseSaved : undefined}
        />
      )}
    </section>
  );
}
