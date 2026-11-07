import type { ComponentChildren } from "preact";
import { useMemo, useState } from "preact/hooks";
import { Link } from "../../shared/router";
import { toast } from "../../shared/toast";
import {
  checkUpdateOnce,
  getIcons,
  getOverview,
  getPreferences,
  getPresences,
  installUpdate,
  savePreferences,
} from "../../platform/api";
import ActivityCard from "../../shared/components/ActivityCard";
import Button from "../../shared/components/Button";
import PageTitle from "../../shared/components/PageTitle";
import StatusIcon, { PriorityBars } from "../../shared/components/StatusIcon";
import { browserList, discordAdvice, discordText } from "../../shared/connection";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { current, useT } from "../../shared/i18n";
import { idleHint, initial } from "../../shared/presence";
import type { PresenceView, Row } from "../../shared/types";
import { deliveryText, problemText, reasonText } from "./reason";
import SetupGuide from "./SetupGuide";
import { optionsLink, ordered, priorityBars, toneOf } from "./tone";
import { pageWidth } from "../../shared/components/ui";

function Banner({ tone, children }: { tone: "warning" | "error"; children: string }) {
  const dot = tone === "warning" ? "bg-warning" : "bg-danger";
  return (
    <div className="mb-3 flex items-start gap-3 rounded-lg border border-neutral-800 bg-neutral-900/60 px-3 py-2.5 text-[13px] text-neutral-300">
      <span className={`mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full ${dot}`} />
      <p className="min-w-0 break-words">{children}</p>
    </div>
  );
}

/** One line of the side panel: a quiet label over a value with its marker. */
function Property({ label, marker, children }: { label: string; marker: ComponentChildren; children: string }) {
  return (
    <div className="py-2.5">
      <p className="mb-1 text-xs text-neutral-600">{label}</p>
      <div className="flex min-w-0 items-center gap-2 text-[13px] text-neutral-200">
        {marker}
        <span className="truncate" title={children}>
          {children}
        </span>
      </div>
    </div>
  );
}

function Dot({ on }: { on: boolean }) {
  return <span className={`h-2 w-2 shrink-0 rounded-full ${on ? "bg-accent" : "bg-neutral-600"}`} />;
}

/** The presence's icon, or its first letter when it has none. */
function Avatar({ name, src }: { name: string; src: string | null }) {
  if (src) return <img src={src} alt="" className="h-4 w-4 shrink-0 rounded object-cover" loading="lazy" />;
  return (
    <span className="flex h-4 w-4 shrink-0 items-center justify-center rounded bg-neutral-800 text-[9px] font-semibold text-neutral-400">
      {initial(name)}
    </span>
  );
}

function PresenceRow({ row, icon }: { row: Row; icon: string | null }) {
  const { t } = useT();
  const reason = reasonText(row.reason, t);
  return (
    <li className="flex h-10 items-center gap-3 px-3 hover:bg-white/[0.025]">
      <StatusIcon tone={toneOf(row.reason, row.selected)} label={reason} />
      <Avatar name={row.label} src={icon} />
      <span className="min-w-0 shrink truncate text-[13px] font-medium text-neutral-100">{row.label}</span>
      <span className="min-w-0 flex-1 truncate text-[13px] text-neutral-500" title={reason}>
        {reason}
      </span>
      <span className="hidden text-xs text-neutral-500 xl:inline">{t(`kind.${row.kind}`)}</span>
      {/* Priority is detail; a narrow window keeps the name and the reason. */}
      <span className="hidden items-center gap-3 sm:flex">
        <PriorityBars filled={priorityBars(row.priority)} label={t("dashboard.priority", { priority: row.priority })} />
        <span className="hidden w-6 text-right text-xs tabular-nums text-neutral-600 xl:inline">{row.priority}</span>
      </span>
      <Link
        to={optionsLink(row.source)}
        className="rounded px-1.5 py-0.5 text-xs text-neutral-500 hover:bg-white/[0.06] hover:text-neutral-100"
        aria-label={t("dashboard.optionsOf", { name: row.label })}
      >
        {t("dashboard.options")}
      </Link>
    </li>
  );
}

function Section({ title, children }: { title: string; children: ComponentChildren }) {
  return (
    <section className="mb-8">
      <h2 className="mb-3 text-[13px] font-medium text-neutral-400">{title}</h2>
      {children}
    </section>
  );
}

/** A list of named problems with their messages. */
function ProblemList({ items }: { items: [string, string][] }) {
  return (
    <ul className="divide-y divide-neutral-800/70 rounded-lg border border-neutral-800 bg-neutral-900/40">
      {items.map(([name, message]) => (
        <li key={name} className="px-3 py-2.5">
          <p className="text-[13px] text-neutral-200">{name}</p>
          <p className="break-words text-xs text-danger">{message}</p>
        </li>
      ))}
    </ul>
  );
}

/** What to do next while no presence has anything to show. */
function NoOffers({ presences }: { presences: PresenceView[] | null }) {
  const { t } = useT();
  const text = "text-[13px] text-neutral-500";
  if (!presences) return <p className={text}>{t("dashboard.noOffers")}</p>;
  const hint = idleHint(presences);
  if (hint.kind === "play") return <p className={text}>{t("dashboard.playOne")}</p>;
  if (hint.kind === "open") {
    const names = new Intl.ListFormat(current(), { type: "disjunction" }).format(hint.names);
    return <p className={text}>{t("dashboard.openOne", { names })}</p>;
  }
  return (
    <p className={text}>
      {t("dashboard.noneOn")}{" "}
      <Link to="/presences" className="text-accent-soft underline underline-offset-2 hover:text-accent-hover">
        {t("dashboard.chooseOne")}
      </Link>
    </p>
  );
}

export default function DashboardPage() {
  const { t } = useT();
  const overview = useLoaded(getOverview, 2000);
  // Read once: names and icons change only when presences are installed or edited.
  const presences = useLoaded(getPresences);
  const localIcons = useLoaded(getIcons);
  // Asked once per session: it goes to the network.
  const update = useLoaded(checkUpdateOnce);
  const [updating, setUpdating] = useState(false);
  const data = overview.data;

  const icons = useMemo(() => {
    const found = new Map<string, string>();
    for (const presence of presences.data ?? []) {
      const src = presence.icon ?? localIcons.data?.[presence.name];
      if (src) found.set(presence.name, src);
    }
    return found;
  }, [presences.data, localIcons.data]);

  async function togglePause() {
    try {
      const current = await getPreferences();
      current.global.paused = !current.global.paused;
      await savePreferences(current);
      overview.reload();
    } catch (error) {
      toast(t("dashboard.pauseFailed"), { description: errorMessage(error) });
    }
  }

  async function install() {
    setUpdating(true);
    try {
      await installUpdate();
    } catch (error) {
      toast(t("update.failed"), { description: errorMessage(error) });
      setUpdating(false);
    }
  }

  if (!data) {
    return <p className="text-sm text-neutral-500">{overview.error ?? t("app.loading")}</p>;
  }
  const { status } = data;
  const rows = ordered(status.rows);
  const shown = rows.find((row) => row.selected) ?? null;
  const advice = discordAdvice(data.discord_running, t);
  const sourceErrors = Object.entries(status.source_errors);

  return (
    <div className={pageWidth}>
      <div className="flex items-start justify-between gap-4">
        <PageTitle title={t("dashboard.title")} subtitle={t("dashboard.subtitle")} />
        <Button variant={data.paused ? "warning" : "ghost"} className="shrink-0 text-xs" onClick={togglePause}>
          {data.paused ? t("dashboard.resume") : t("dashboard.pause")}
        </Button>
      </div>

      {presences.data && (
        <SetupGuide
          discordRunning={data.discord_running}
          extensions={data.extensions}
          enabled={presences.data.filter((presence) => presence.enabled).length}
          offering={rows.length}
        />
      )}
      {update.data?.version && (
        <div className="mb-3 flex items-center justify-between gap-3 rounded-lg border border-neutral-800 bg-neutral-900/60 px-3 py-2">
          <p className="text-[13px] text-neutral-200">{t("update.available", { version: update.data.version })}</p>
          <Button variant="primary" className="text-xs" loading={updating} onClick={install}>
            {t("update.install")}
          </Button>
        </div>
      )}
      {data.warning && <Banner tone="warning">{data.warning}</Banner>}
      {advice && <Banner tone="warning">{advice}</Banner>}
      {(status.blank_sources ?? []).map((name) => (
        <Banner key={name} tone="warning">
          {t("dashboard.blank", { name })}
        </Banner>
      ))}
      {status.delivery_error && (
        <Banner tone="error">{deliveryText(status.delivery_error, t)}</Banner>
      )}

      <div className="mt-6 grid gap-8 md:grid-cols-[minmax(0,1fr)_200px]">
        <div className="min-w-0">
          <Section title={t("dashboard.nowShowing")}>
            {data.paused ? (
              <p className="rounded-lg border border-neutral-800 bg-neutral-900/40 p-4 text-[13px] text-neutral-500">
                {t("dashboard.paused")}
              </p>
            ) : (
              <ActivityCard
                activity={status.activity}
                title={shown?.label ?? ""}
                logo={shown ? icons.get(shown.source) : null}
              />
            )}
          </Section>

          <Section title={t("dashboard.why")}>
            {rows.length === 0 ? (
              <NoOffers presences={data.paused ? null : presences.data} />
            ) : (
              <ul className="divide-y divide-neutral-800/70 overflow-hidden rounded-lg border border-neutral-800 bg-neutral-900/40">
                {rows.map((row) => (
                  <PresenceRow key={row.source} row={row} icon={icons.get(row.source) ?? null} />
                ))}
              </ul>
            )}
          </Section>

          {sourceErrors.length > 0 && (
            <Section title={t("dashboard.problems")}>
              <ProblemList items={sourceErrors.map(([name, message]) => [name, problemText(message, t)])} />
            </Section>
          )}
          {data.problems.length > 0 && (
            <Section title={t("dashboard.notLoaded")}>
              <ProblemList items={data.problems.map((problem) => [problem.folder, problem.message])} />
            </Section>
          )}
        </div>

        <aside className="h-fit divide-y divide-neutral-800/70 md:border-l md:border-neutral-800 md:pl-6">
          <Property
            label={t("dashboard.publishing")}
            marker={<StatusIcon tone={data.paused ? "paused" : shown ? "showing" : "standby"} label="" />}
          >
            {data.paused ? t("dashboard.paused") : (shown?.label ?? t("dashboard.nothing"))}
          </Property>
          <Property label={t("dashboard.discord")} marker={<Dot on={data.discord_running} />}>
            {discordText(data.discord_running, data.discord_user, t)}
          </Property>
          <Property label={t("dashboard.browser")} marker={<Dot on={data.browsers.length > 0} />}>
            {data.browsers.length > 0 ? browserList(data.browsers, t) : t("dashboard.noExtension")}
          </Property>
        </aside>
      </div>
    </div>
  );
}
