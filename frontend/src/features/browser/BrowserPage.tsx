import type { ComponentChildren } from "preact";
import { useEffect, useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { getExtensionSessions, getPairing } from "../../platform/api";
import Accordion from "../../shared/components/Accordion";
import Button from "../../shared/components/Button";
import PageTitle from "../../shared/components/PageTitle";
import { browserMark, browserName, duration } from "../../shared/connection";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import type { SessionView } from "../../shared/types";
import { pageWidth } from "../../shared/components/ui";

/** Unix seconds now, refreshed every 30 s so durations move without polling the app. */
function useSeconds(): number {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => {
    const timer = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 30_000);
    return () => clearInterval(timer);
  }, []);
  return now;
}

function Section({ title, hint, children }: { title: string; hint?: string; children: ComponentChildren }) {
  return (
    <section className="mb-8">
      <h2 className="text-[13px] font-medium text-neutral-400">{title}</h2>
      {hint && <p className="mt-0.5 text-xs text-neutral-600">{hint}</p>}
      <div className="mt-3">{children}</div>
    </section>
  );
}

/** The browser's initial on its own tint; the makers' logos need their permission. */
function Badge({ browser, live }: { browser: string; live: boolean }) {
  const { letter, tint } = browserMark(browser);
  return (
    <span
      aria-hidden="true"
      className="relative flex h-8 w-8 shrink-0 items-center justify-center rounded-md border text-[13px] font-semibold"
      style={{ backgroundColor: `${tint}26`, borderColor: `${tint}66`, color: tint }}
    >
      {letter}
      {live && (
        <span className="absolute -right-0.5 -bottom-0.5 h-2.5 w-2.5 rounded-full border-2 border-neutral-950 bg-accent" />
      )}
    </span>
  );
}

function SessionRow({ session, now }: { session: SessionView; now: number }) {
  const { t, tn, tag } = useT();
  const clock = (seconds: number) =>
    new Date(seconds * 1000).toLocaleTimeString(tag, { hour: "2-digit", minute: "2-digit" });
  const name = browserName(session.browser, t);
  const end = session.ended_at;
  const when =
    end === null
      ? t("browser.connectedFor", { time: clock(session.connected_at), duration: duration(now - session.connected_at, t) })
      : t("browser.span", {
          from: clock(session.connected_at),
          to: clock(end),
          duration: duration(end - session.connected_at, t),
        });
  return (
    <li className="flex items-center gap-3 px-3 py-2.5">
      <Badge browser={session.browser} live={end === null} />
      <div className="min-w-0 flex-1">
        <p className="truncate text-[13px] text-neutral-100">
          <span className="font-medium">{name}</span>
          <span className="ml-2 text-neutral-500">{session.version || t("browser.unknownVersion")}</span>
          {session.outdated && end === null && <span className="ml-2 text-warning">{t("browser.outdated")}</span>}
        </p>
        <p className="truncate text-xs text-neutral-500">{when}</p>
      </div>
      <span className="shrink-0 text-xs tabular-nums text-neutral-500">{tn("browser.tabs", session.tabs)}</span>
    </li>
  );
}

function SessionList({ sessions, empty, now }: { sessions: SessionView[]; empty: string; now: number }) {
  if (sessions.length === 0) return <p className="text-[13px] text-neutral-500">{empty}</p>;
  return (
    <ul className="divide-y divide-neutral-800/70 rounded-lg border border-neutral-800 bg-neutral-900/40">
      {sessions.map((session) => (
        <SessionRow key={`${session.browser}-${session.connected_at}-${session.ended_at}`} session={session} now={now} />
      ))}
    </ul>
  );
}

export default function BrowserPage() {
  const { t } = useT();
  const pairing = useLoaded(getPairing);
  const sessions = useLoaded(getExtensionSessions, 2000);
  const now = useSeconds();
  const [shown, setShown] = useState(false);
  const code = pairing.data?.code ?? "";
  const live = sessions.data?.live ?? [];

  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
      toast(t("browser.copied"));
    } catch {
      toast(t("browser.copyFailed"));
    }
  }

  return (
    <div className={pageWidth}>
      <PageTitle title={t("browser.title")} subtitle={t("browser.subtitle")} />

      <Section title={t("browser.live")}>
        <SessionList sessions={live} empty={t("browser.noLive")} now={now} />
        {live.some((session) => session.outdated) && (
          <p className="mt-2 text-xs text-neutral-400">{t("browser.outdatedHint")}</p>
        )}
      </Section>

      <Section title={t("browser.recent")} hint={t("browser.recentHint")}>
        <SessionList sessions={sessions.data?.recent ?? []} empty={t("browser.noRecent")} now={now} />
      </Section>

      <Accordion className="mb-6" open={live.length === 0} title={t("browser.manual")}>
        <ol className="list-inside list-decimal space-y-4 text-[13px] text-neutral-300">
          <li>{t("browser.step1")}</li>
          <li>
            {t("browser.step2")}
            <div className="mt-2 flex items-center gap-2">
              <code className="min-w-0 flex-1 truncate rounded-md border border-neutral-800 bg-neutral-950 px-3 py-2 text-xs text-neutral-300">
                {shown ? code : code.replace(/:.*/, ":" + "•".repeat(16))}
              </code>
              <Button variant="ghost" className="text-xs" onClick={() => setShown(!shown)}>
                {shown ? t("browser.hide") : t("browser.show")}
              </Button>
              <Button variant="primary" className="text-xs" onClick={copy} disabled={!code}>
                {t("browser.copy")}
              </Button>
            </div>
          </li>
          <li>{t("browser.step3")}</li>
        </ol>
        <p className="mt-3 text-xs text-warning">{t("browser.warning")}</p>
      </Accordion>
    </div>
  );
}
