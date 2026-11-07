import { useState } from "preact/hooks";
import { clearUsage, exportUsage, getUsage, setTrackTime } from "../../platform/api";
import Button from "../../shared/components/Button";
import PageTitle from "../../shared/components/PageTitle";
import Switch from "../../shared/components/Switch";
import { Panel, Section, SettingRow, pageWidth } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { toast } from "../../shared/toast";
import { type WeekDay, hoursAndMinutes, lastWeek, namesOfTheWeek } from "./week";

/** How often the page reads the count while it is open. */
const REFRESH_MS = 30_000;

/** Seven bars, one per day, with the time of the day under the pointer. */
function WeekChart({ week, duration }: { week: WeekDay[]; duration: (seconds: number) => string }) {
  const { tag } = useT();
  const [pointed, setPointed] = useState<WeekDay | null>(null);
  const top = Math.max(...week.map((day) => day.seconds), 1);
  const weekday = (date: string) => new Intl.DateTimeFormat(tag, { weekday: "short" }).format(new Date(`${date}T12:00:00`));
  const shown = pointed ?? week[week.length - 1];

  return (
    <div className="p-4">
      <p className="mb-3 text-[13px] text-neutral-400">
        {weekday(shown.date)}: <span className="text-neutral-100">{duration(shown.seconds)}</span>
      </p>
      <div className="flex h-32 items-end gap-0.5">
        {week.map((day) => (
          <button
            key={day.date}
            type="button"
            className="group flex h-full min-w-0 flex-1 flex-col items-center justify-end gap-1 bg-transparent"
            aria-label={`${weekday(day.date)}: ${duration(day.seconds)}`}
            onMouseEnter={() => setPointed(day)}
            onMouseLeave={() => setPointed(null)}
            onFocus={() => setPointed(day)}
            onBlur={() => setPointed(null)}
          >
            <span
              className="w-full max-w-10 rounded-t bg-accent opacity-80 group-hover:opacity-100"
              style={{ height: `${day.seconds === 0 ? 0 : Math.max((day.seconds / top) * 85, 2)}%` }}
            />
            <span className="text-[11px] text-neutral-500">{weekday(day.date)}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

/** How long each presence stayed on display, kept on this computer. */
export default function WeekPage() {
  const { t } = useT();
  const usage = useLoaded(getUsage, REFRESH_MS);
  const [confirming, setConfirming] = useState(false);

  if (!usage.data) return <p className="text-sm text-neutral-500">{usage.error ?? t("app.loading")}</p>;
  const { on, days } = usage.data;

  const today = new Date();
  const week = lastWeek(today, days);
  const names = namesOfTheWeek(today, days);
  const total = week.reduce((sum, day) => sum + day.seconds, 0);
  const most = names[0]?.seconds ?? 1;

  function duration(seconds: number) {
    const { hours, minutes } = hoursAndMinutes(seconds);
    return hours > 0 ? t("week.hours", { hours, minutes }) : t("week.minutes", { minutes });
  }

  async function act(action: () => Promise<void>) {
    try {
      await action();
      usage.reload();
    } catch (error) {
      toast(t("week.failed"), { description: errorMessage(error) });
    }
  }

  async function copy() {
    await act(async () => {
      await navigator.clipboard.writeText(await exportUsage());
      toast(t("week.copied"));
    });
  }

  async function clear() {
    setConfirming(false);
    await act(async () => {
      await clearUsage();
      toast(t("week.cleared"));
    });
  }

  return (
    <div className={pageWidth}>
      <PageTitle title={t("week.title")} subtitle={t("week.subtitle")} />
      <Section title={t("week.counting")}>
        <Panel>
          <SettingRow title={t("week.count")} hint={t("week.countHint")}>
            <Switch checked={on} onChange={(value) => act(() => setTrackTime(value))} label={t("week.count")} />
          </SettingRow>
        </Panel>
      </Section>

      {on && total === 0 && <p className="mb-6 text-[13px] text-neutral-500">{t("week.empty")}</p>}

      {total > 0 && (
        <>
          <Section title={t("week.total", { time: duration(total) })}>
            <Panel>
              <WeekChart week={week} duration={duration} />
            </Panel>
          </Section>
          <Section title={t("week.byPresence")}>
            <Panel>
              {names.map(({ name, seconds }) => (
                <div key={name} className="px-4 py-2.5">
                  <div className="flex items-baseline justify-between gap-3 text-[13px]">
                    <span className="truncate text-neutral-200">{name}</span>
                    <span className="shrink-0 text-neutral-400">{duration(seconds)}</span>
                  </div>
                  <div className="mt-1.5 h-1 overflow-hidden rounded bg-neutral-800">
                    <div className="h-full rounded bg-accent" style={{ width: `${Math.max((seconds / most) * 100, 1)}%` }} />
                  </div>
                </div>
              ))}
            </Panel>
          </Section>
          <div className="flex flex-wrap gap-2">
            <Button variant="neutral" onClick={copy}>
              {t("week.copy")}
            </Button>
            {confirming ? (
              <Button variant="primary" onClick={clear}>
                {t("week.clearSure")}
              </Button>
            ) : (
              <Button variant="ghost" onClick={() => setConfirming(true)}>
                {t("week.clear")}
              </Button>
            )}
          </div>
        </>
      )}
    </div>
  );
}
