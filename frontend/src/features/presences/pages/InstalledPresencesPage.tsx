import { useEffect, useState, useMemo } from "preact/hooks";
import {
  getInstalledPresences,
  startPresence,
  stopPresence,
} from "../../../platform/pywebview/presences.api";
import type { InstalledPresence } from "../../../shared/types/presence";

export default function InstalledPresencesPage() {
  const [presences, setPresences] = useState<InstalledPresence[]>([]);
  const [searchQuery, setSearchQuery] = useState("");

  const filteredPresences = useMemo(() => {
    if (!searchQuery.trim()) return presences;
    const query = searchQuery.toLowerCase();
    return presences.filter(
      (p) =>
        p.name.toLowerCase().includes(query) ||
        p.client_id.toLowerCase().includes(query),
    );
  }, [presences, searchQuery]);

  const handleStart = async (name: string) => {
    try {
      let presenceInstance = presences.find((p) => p.name === name);
      if (presenceInstance && presenceInstance.running) {
        alert("Presence is already running.");
        return;
      }
      await startPresence(name);
      // change running state
      setPresences((prevPresences) =>
        prevPresences.map((p) =>
          p.name === name ? { ...p, running: true } : p,
        ),
      );
    } catch (error) {
      console.error("Error launching presence:", error);
      alert(String(error));
    }
  };
  const handleStop = async (name: string) => {
    try {
      let presenceInstance = presences.find((p) => p.name === name);
      if (presenceInstance && !presenceInstance.running) {
        alert("Presence is not running.");
        return;
      }
      await stopPresence(name);
      // change running state
      setPresences((prevPresences) =>
        prevPresences.map((p) =>
          p.name === name ? { ...p, running: false } : p,
        ),
      );
    } catch (error) {
      console.error("Error stopping presence:", error);
      alert(String(error));
    }
  };

  useEffect(() => {
    async function fetchPresences() {
      try {
        const installedPresences = await getInstalledPresences();
        console.log("Installed presences:", installedPresences);
        setPresences(installedPresences);
      } catch (error) {
        console.error("Error fetching installed presences:", error);
      }
    }

    fetchPresences();
  }, []);
  return (
    <div className="max-w-4xl">
      <div className="mb-8">
        <h1 className="text-2xl font-semibold text-white tracking-tight">
          Presences
        </h1>
        <p className="text-neutral-400 mt-2 text-base">
          {presences.length} installed
        </p>
      </div>

      <div className="mb-6">
        <div className="relative">
          <svg
            className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-neutral-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            type="text"
            placeholder="Search presences..."
            value={searchQuery}
            onChange={(e) =>
              setSearchQuery((e.target as HTMLInputElement).value)
            }
            className="w-full pl-12 pr-4 py-3 rounded-xl bg-neutral-900/50 border border-neutral-800 text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-green-500/50 focus:ring-1 focus:ring-green-500/20"
          />
        </div>
      </div>

      {presences.length === 0 ? (
        <div className="p-8 rounded-xl bg-neutral-900/50 border border-neutral-800 text-center">
          <p className="text-base text-neutral-400">No presences installed</p>
          <p className="text-sm text-neutral-500 mt-1">
            Visit the Store to install presences.
          </p>
        </div>
      ) : filteredPresences.length === 0 ? (
        <div className="p-8 rounded-xl bg-neutral-900/50 border border-neutral-800 text-center">
          <p className="text-base text-neutral-400">No results found</p>
          <p className="text-sm text-neutral-500 mt-1">
            Try a different search term
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {filteredPresences.map((presence, index) => (
            <div
              key={index}
              className="flex items-center justify-between p-4 rounded-xl bg-neutral-900/30 border border-neutral-800/50 hover:border-neutral-700/50"
            >
              <div className="flex items-center gap-4 min-w-0">
                <div
                  className={`w-3 h-3 rounded-full ${
                    presence.running ? "bg-green-500" : "bg-neutral-600"
                  }`}
                />
                <div className="min-w-0">
                  <p className="text-base text-neutral-200 truncate">
                    {presence.name}
                  </p>
                  <div className="flex items-center gap-3 text-sm text-neutral-500">
                    <span className="font-mono">{presence.client_id}</span>
                    {presence.web && (
                      <span className="px-2 py-0.5 rounded bg-neutral-800 text-neutral-400 text-xs">
                        web
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-3">
                {presence.running ? (
                  <button
                    onClick={() => handleStop(presence.name)}
                    className="px-4 py-2 text-sm font-medium rounded-lg bg-amber-500/10 text-amber-400 hover:bg-amber-500/20"
                  >
                    Stop
                  </button>
                ) : (
                  <button
                    onClick={() => handleStart(presence.name)}
                    className="px-4 py-2 text-sm font-medium rounded-lg bg-green-500/10 text-green-400 hover:bg-green-500/20"
                  >
                    Start
                  </button>
                )}
                <button className="px-4 py-2 text-sm font-medium rounded-lg bg-neutral-800 text-neutral-400 hover:bg-neutral-700">
                  Remove
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
