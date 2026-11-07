import { useEffect, useState } from "preact/hooks";
import { useT } from "../i18n";
import type { Activity } from "../types";

interface Props {
  /** What the card shows; null draws an empty card. */
  activity: Activity | null;
  /** Name of the Discord application, shown as the title. */
  title: string;
  /** Logo of the presence, for an activity with no image of its own. */
  logo?: string | null;
}

/** Only web images are drawn: an asset key names a file inside Discord. */
function picture(source: string | null): string | null {
  return source && /^https:\/\//.test(source) ? source : null;
}

/** `75` becomes `1:15`; an hour or more becomes `1:02:03`. */
export function clock(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

/** The current time in seconds, refreshed every second while `active`. */
function useNow(active: boolean): number {
  const [now, setNow] = useState(() => Date.now() / 1000);
  useEffect(() => {
    if (!active) return;
    setNow(Date.now() / 1000);
    const timer = setInterval(() => setNow(Date.now() / 1000), 1000);
    return () => clearInterval(timer);
  }, [active]);
  return now;
}

function Timeline({ start, end }: { start: number | null; end: number | null }) {
  const { t } = useT();
  const now = useNow(start !== null);
  if (start === null) return null;
  if (end === null) {
    return <p className="text-xs text-accent-soft">{t("card.elapsed", { time: clock(now - start) })}</p>;
  }
  const length = Math.max(1, end - start);
  const position = Math.min(length, Math.max(0, now - start));
  return (
    <div className="flex items-center gap-2 text-[11px] text-neutral-400">
      <span>{clock(position)}</span>
      <div className="h-1 flex-1 rounded bg-neutral-700">
        <div className="h-1 rounded bg-neutral-300" style={{ width: `${(position / length) * 100}%` }} />
      </div>
      <span>{clock(length)}</span>
    </div>
  );
}

/** An activity drawn like Discord draws it on a profile. */
export default function ActivityCard({ activity, title, logo = null }: Props) {
  const { t } = useT();
  if (!activity) {
    return (
      <p className="rounded-lg border border-dashed border-neutral-800 p-4 text-[13px] text-neutral-500">{t("card.empty")}</p>
    );
  }
  const large = picture(activity.large_image);
  const small = picture(activity.small_image);
  return (
    <div className="rounded-lg border border-neutral-800 bg-neutral-900/60 p-4 text-neutral-200">
      <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-neutral-400">{title}</p>
      <div className="flex gap-3">
        <div className="relative h-16 w-16 shrink-0">
          {large ? (
            <img src={large} alt="" className="h-16 w-16 rounded-lg object-cover" />
          ) : logo ? (
            <img src={logo} alt="" className="h-16 w-16 rounded-lg bg-neutral-800 object-contain p-2.5" />
          ) : (
            <div className="h-16 w-16 rounded-lg bg-neutral-800" />
          )}
          {small && (
            <img
              src={small}
              alt=""
              className="absolute -bottom-1 -right-1 h-6 w-6 rounded-full border-2 border-neutral-900 object-cover"
            />
          )}
        </div>
        <div className="min-w-0 flex-1 space-y-0.5 text-sm">
          {activity.name && <p className="truncate font-semibold">{activity.name}</p>}
          {activity.details && <p className="truncate font-medium">{activity.details}</p>}
          {activity.state && <p className="truncate text-neutral-300">{activity.state}</p>}
          <Timeline start={activity.start} end={activity.end} />
        </div>
      </div>
      {activity.buttons.length > 0 && (
        <div className="mt-3 grid gap-2">
          {activity.buttons.slice(0, 2).map((button) => (
            <span
              key={`${button.label}${button.url}`}
              className="truncate rounded bg-neutral-700 px-3 py-1.5 text-center text-sm font-medium"
            >
              {button.label}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
