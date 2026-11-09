import type { ComponentChildren } from "preact";
import { Suspense } from "preact/compat";
import { useRef } from "preact/hooks";
import { getOverview } from "./platform/api";
import FirstRun from "./shared/components/FirstRun";
import PageGuard from "./shared/components/PageGuard";
import Toaster from "./shared/components/Toaster";
import { inSection } from "./shared/navigation";
import { useLoaded } from "./shared/hooks";
import { useT } from "./shared/i18n";
import { Link, useLocation } from "./shared/router";

// What people use most comes first, then what sets it up. `gap` starts a group.
const navItems = [
  {
    label: "nav.dashboard",
    path: "/",
    icon: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6",
  },
  {
    label: "nav.presences",
    path: "/presences",
    icon: "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10",
  },
  {
    label: "nav.manual",
    path: "/manual",
    icon: "M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z",
  },
  {
    label: "nav.week",
    path: "/week",
    icon: "M9 19v-6m6 6V5m6 14v-9M3 19v-3",
  },
  {
    gap: true,
    label: "nav.browser",
    path: "/browser",
    icon: "M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9",
  },
  {
    label: "nav.privacy",
    path: "/privacy",
    icon: "M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z",
  },
  {
    label: "nav.settings",
    path: "/settings",
    icon: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065zM15 12a3 3 0 11-6 0 3 3 0 016 0z",
  },
  {
    gap: true,
    label: "nav.developer",
    path: "/developer",
    icon: "M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4",
  },
];

function Icon({ path }: { path: string }) {
  return (
    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d={path} />
    </svg>
  );
}

export default function Layout({ children }: { children: ComponentChildren }) {
  const { t } = useT();
  const navigation = useRef<HTMLDialogElement>(null);
  const location = useLocation();
  // Only the version is shown here, and it does not change while running.
  const overview = useLoaded(getOverview);
  const version = overview.data?.version ?? "";

  return (
    <div className="w-full h-dvh min-h-0 flex flex-col md:flex-row bg-neutral-950 text-neutral-100">
      <aside className="hidden md:flex w-52 lg:w-56 min-w-52 bg-neutral-900 flex-col border-r border-neutral-800">
        <div className="px-4 lg:px-5 py-4">
          <div className="flex items-center gap-3">
            <img src="/logo.svg" alt="" className="h-7 w-7" />
            <span className="text-sm font-semibold tracking-tight text-neutral-100">
              {t("app.name")}<span className="text-accent"> +</span>
            </span>
          </div>
        </div>
        <nav className="flex-1 px-2 lg:px-3 py-3 lg:py-4 overflow-y-auto">
          <ul className="space-y-0.5">
            {navItems.map((item) => {
              const active = inSection(location.pathname, item.path);
              return (
                <li key={item.path} className={item.gap ? "pt-3" : undefined}>
                  <Link
                    to={item.path}
                    aria-current={active ? "page" : undefined}
                    className={`flex items-center gap-2.5 px-2.5 py-1.5 rounded-md text-[13px] font-medium ${
                      active
                        ? "bg-white/[0.06] text-neutral-100"
                        : "text-neutral-400 hover:text-neutral-100 hover:bg-white/[0.03]"
                    }`}
                  >
                    <span className={`shrink-0 ${active ? "text-neutral-200" : "text-neutral-500"}`}>
                      <Icon path={item.icon} />
                    </span>
                    <span className="truncate">{t(item.label)}</span>
                  </Link>
                </li>
              );
            })}
          </ul>
        </nav>
        <div className="px-4 lg:px-5 py-3 lg:py-4 border-t border-neutral-800 space-y-3">
          <p className="text-[11px] text-neutral-600">{version && `v${version}`}</p>
        </div>
      </aside>

      <header className="md:hidden flex items-center justify-between px-4 py-3 bg-neutral-900 border-b border-neutral-800">
        <button
          className="min-h-11 px-3 rounded-lg border border-neutral-700 text-sm"
          aria-label={t("nav.open")}
          onClick={() => navigation.current?.showModal()}
        >
          {t("nav.menu")}
        </button>
        <span className="text-sm font-semibold tracking-tight">
          {t("app.name")}<span className="text-accent"> +</span>
        </span>
        <span className="text-[10px] text-neutral-600">{version && `v${version}`}</span>
      </header>
      <dialog
        ref={navigation}
        className="m-auto max-h-[90dvh] w-80 max-w-[90vw] overflow-auto rounded-lg border border-neutral-800 bg-neutral-900 p-4 text-neutral-100 shadow-2xl"
        aria-label={t("nav.label")}
      >
        <button className="min-h-11 px-3 mb-3 text-sm text-neutral-300" onClick={() => navigation.current?.close()}>
          {t("nav.close")}
        </button>
        <nav className="flex flex-col gap-1">
          {navItems.map((item) => (
            <Link
              key={item.path}
              to={item.path}
              onClick={() => navigation.current?.close()}
              aria-current={inSection(location.pathname, item.path) ? "page" : undefined}
              className={`min-h-11 flex items-center gap-3 px-3 rounded-lg text-sm ${
                inSection(location.pathname, item.path) ? "bg-white/[0.06] text-neutral-100" : "text-neutral-300 hover:bg-neutral-800"
              }`}
            >
              <Icon path={item.icon} />
              {t(item.label)}
            </Link>
          ))}
        </nav>
      </dialog>

      <Toaster />
      <main className="flex-1 min-w-0 min-h-0 overflow-auto [scrollbar-gutter:stable] p-4 md:p-5 lg:p-6 bg-neutral-900/30">
        <PageGuard key={location.pathname}>
          <Suspense fallback={null}>{children}</Suspense>
        </PageGuard>
      </main>
      <FirstRun />
    </div>
  );
}
