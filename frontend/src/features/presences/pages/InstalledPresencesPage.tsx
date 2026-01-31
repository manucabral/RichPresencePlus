import { useEffect, useState, useMemo } from "preact/hooks";
import {
  getInstalledPresences,
  removeInstalledPresence,
  startPresence,
  stopPresence,
} from "../../../platform/pywebview/presences.api";
import type { InstalledPresence } from "../../../shared/types/presence";
import { toast } from "sonner";
import Button from "../../../shared/components/Button";

export default function InstalledPresencesPage() {
  const [isLoading, setIsLoading] = useState(true);
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

  const getPresenceByName = (name: string) => {
    return presences.find((p) => p.name === name);
  };

  const handleRemove = async (name: string) => {
    try {
      let presence = getPresenceByName(name);
      if (presence && presence.running) {
        toast("Error", {
          description: "Please stop the presence before removing it.",
          duration: 5000,
        });
        return;
      }
      setIsLoading(true);

      const { success, message } = await removeInstalledPresence(name);
      toast(success ? "Removed" : "Error", {
        description: message,
        duration: 5000,
      });
      if (!success) return;
      setPresences((prevPresences) =>
        prevPresences.filter((p) => p.name !== name),
      );
    } catch (error) {
      toast("Error", {
        description: String(error),
        duration: 5000,
      });
    } finally {
      setIsLoading(false);
    }
  };

  const handleStart = async (name: string) => {
    try {
      setIsLoading(true);
      let presenceInstance = getPresenceByName(name);
      if (presenceInstance && presenceInstance.running) {
        toast("Error", {
          description: "Presence is already running.",
          duration: 5000,
        });
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
      toast("Cannot start", {
        description: String(error),
        duration: 5000,
      });
    } finally {
      setIsLoading(false);
    }
  };
  const handleStop = async (name: string) => {
    try {
      setIsLoading(true);
      let presenceInstance = getPresenceByName(name);
      if (presenceInstance && !presenceInstance.running) {
        toast("Cannot stop", {
          description: "Presence is not running.",
          duration: 5000,
        });
        return;
      }
      await stopPresence(name);
      setPresences((prevPresences) =>
        prevPresences.map((p) =>
          p.name === name ? { ...p, running: false } : p,
        ),
      );
    } catch (error) {
      toast("Cannot stop", {
        description: String(error),
        duration: 5000,
      });
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    async function fetchPresences() {
      try {
        setIsLoading(true);
        const installedPresences = await getInstalledPresences();
        console.log("Installed presences:", installedPresences);
        setPresences(installedPresences);
      } catch (error) {
        toast("Error", {
          description: "Failed to fetch installed presences.",
          duration: 5000,
        });
      } finally {
        setIsLoading(false);
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
                {presence.image ? (
                  <img
                    src={presence.image}
                    alt={presence.name}
                    className="w-10 h-10 rounded-lg object-cover"
                  />
                ) : (
                  <div className="w-10 h-10 rounded-lg flex items-center justify-center bg-neutral-800 text-neutral-400 font-bold">
                    {presence.name.charAt(0).toUpperCase()}
                  </div>
                )}
                <div className="min-w-0">
                  <p className="text-base text-neutral-200 truncate">
                    {presence.name}
                  </p>
                  <div className="flex flex-col gap-1">
                    <div className="flex items-center gap-3 text-sm text-neutral-500">
                      <span className="font-mono">{presence.client_id}</span>
                      {presence.web && (
                        <span className="px-2 py-0.5 rounded bg-neutral-800 text-neutral-400 text-xs">
                          web
                        </span>
                      )}
                      <span
                        className={`px-2 py-0.5 rounded text-xs ${
                          presence.verified
                            ? "bg-green-500/10 text-green-400"
                            : "bg-red-500/10 text-red-400"
                        }`}
                      >
                        {presence.verified ? "verified" : "unverified"}
                      </span>
                    </div>
                    {presence.path && (
                      <div
                        className="text-xs text-neutral-600 font-mono truncate max-w-xs"
                        title={presence.path}
                      >
                        {presence.path}
                      </div>
                    )}
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-3">
                {presence.running ? (
                  <Button
                    onClick={() => handleStop(presence.name)}
                    variant="warning"
                    disabled={isLoading}
                  >
                    Stop
                  </Button>
                ) : (
                  <Button
                    onClick={() => handleStart(presence.name)}
                    variant="primary"
                    disabled={isLoading}
                  >
                    Start
                  </Button>
                )}
                <Button
                  onClick={() => handleRemove(presence.name)}
                  variant="neutral"
                  disabled={isLoading}
                >
                  Remove
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
