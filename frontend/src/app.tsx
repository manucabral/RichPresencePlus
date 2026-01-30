import { Link } from "react-router";
import { useState, useEffect } from "preact/hooks";
import { getConnectedBrowser } from "./platform/pywebview/browser.api";
import { isDiscordRunning } from "./platform/pywebview/presences.api";
import type { Browser } from "./shared/types/browser";

const quickLinks = [
  {
    label: "Browser",
    path: "/browser",
    desc: "Configure browser connection",
    icon: (
      <svg
        className="w-5 h-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={1.5}
          d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"
        />
      </svg>
    ),
  },
  {
    label: "Presences",
    path: "/installed-presences",
    desc: "Manage installed presences",
    icon: (
      <svg
        className="w-5 h-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={1.5}
          d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
        />
      </svg>
    ),
  },
  {
    label: "Store",
    path: "/remote-presences",
    desc: "Browse available presences",
    icon: (
      <svg
        className="w-5 h-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={1.5}
          d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
        />
      </svg>
    ),
  },
  {
    label: "Network",
    path: "/network",
    desc: "Monitor active processes",
    icon: (
      <svg
        className="w-5 h-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={1.5}
          d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"
        />
      </svg>
    ),
  },
  {
    label: "Discord",
    path: "/discord",
    desc: "Check Discord status",
    icon: (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
        <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
      </svg>
    ),
  },
];

export default function App() {
  const [discordRunning, setDiscordRunning] = useState<boolean | null>(null);
  const [browser, setBrowser] = useState<Browser | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  const checkStatus = async () => {
    setIsLoading(true);
    try {
      const [discordResult, browserResult] = await Promise.all([
        isDiscordRunning(),
        getConnectedBrowser(),
      ]);
      setDiscordRunning(discordResult);
      setBrowser(browserResult);
    } catch (error) {
      console.error("Error checking status:", error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    checkStatus();
  }, []);

  const browserConnected = browser !== null;

  return (
    <div className="max-w-2xl">
      <div className="mb-8">
        <h1 className="text-xl font-semibold text-neutral-100 tracking-tight">
          Dashboard
        </h1>
        <p className="text-neutral-500 mt-1 text-sm">
          Manage your Discord presence
        </p>
      </div>

      <div className="mb-8 p-5 rounded-xl bg-neutral-900/50 border border-neutral-800">
        <div className="flex items-center justify-between mb-5">
          <h2 className="text-sm font-medium text-neutral-400 uppercase tracking-wider">
            System Status
          </h2>
          <button
            onClick={checkStatus}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium rounded-lg bg-neutral-800 text-neutral-400 hover:text-green-400 hover:bg-neutral-800/80 disabled:opacity-50 disabled:cursor-not-allowed border border-neutral-700/50"
            title="Refresh Status"
          >
            <svg
              className={`w-3.5 h-3.5 ${isLoading ? "animate-spin" : ""}`}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              />
            </svg>
            <span className="hidden sm:inline">
              {isLoading ? "Refreshing..." : "Refresh"}
            </span>
          </button>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div className="flex items-center gap-3 p-4 rounded-lg bg-neutral-800/40 border border-neutral-700/30">
            <div
              className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0 ${discordRunning ? "bg-green-500/10" : "bg-neutral-800"}`}
            >
              <svg
                className={`w-5 h-5 ${discordRunning ? "text-green-400" : "text-neutral-500"}`}
                fill="currentColor"
                viewBox="0 0 24 24"
              >
                <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
              </svg>
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-neutral-200">Discord</p>
              {isLoading ? (
                <p className="text-xs text-neutral-500">Checking...</p>
              ) : (
                <p
                  className={`text-xs ${discordRunning ? "text-green-400" : "text-red-400"}`}
                >
                  {discordRunning ? "Running" : "Not Running"}
                </p>
              )}
            </div>
            <div
              className={`w-2.5 h-2.5 rounded-full shrink-0 ${isLoading ? "bg-neutral-600 animate-pulse" : discordRunning ? "bg-green-500" : "bg-red-500"}`}
            />
          </div>

          <div className="flex items-center gap-3 p-4 rounded-lg bg-neutral-800/40 border border-neutral-700/30">
            <div
              className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0 ${browserConnected ? "bg-green-500/10" : "bg-neutral-800"}`}
            >
              <svg
                className={`w-5 h-5 ${browserConnected ? "text-green-400" : "text-neutral-500"}`}
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"
                />
              </svg>
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-neutral-200">Browser</p>
              {isLoading ? (
                <p className="text-xs text-neutral-500">Checking...</p>
              ) : (
                <p
                  className={`text-xs truncate ${browserConnected ? "text-green-400" : "text-neutral-500"}`}
                >
                  {browserConnected
                    ? browser?.name || "Connected"
                    : "Not Connected"}
                </p>
              )}
            </div>
            <div
              className={`w-2.5 h-2.5 rounded-full shrink-0 ${isLoading ? "bg-neutral-600 animate-pulse" : browserConnected ? "bg-green-500" : "bg-neutral-600"}`}
            />
          </div>
        </div>
      </div>

      <div>
        <h2 className="text-sm font-medium text-neutral-400 uppercase tracking-wider mb-4">
          Quick Links
        </h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {quickLinks.map((link) => (
            <Link
              key={link.path}
              to={link.path}
              className="group flex items-start gap-3 p-4 rounded-xl bg-neutral-900/30 border border-neutral-800/50 hover:border-green-500/30 hover:bg-neutral-900/60"
            >
              <span className="text-neutral-600 group-hover:text-green-400 shrink-0">
                {link.icon}
              </span>
              <div className="min-w-0">
                <p className="text-sm font-medium text-neutral-200 group-hover:text-green-400">
                  {link.label}
                </p>
                <p className="text-xs text-neutral-600 mt-0.5 truncate">
                  {link.desc}
                </p>
              </div>
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}
