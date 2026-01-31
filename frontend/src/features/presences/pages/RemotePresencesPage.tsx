import { useEffect, useState, useMemo } from "preact/hooks";
import { toast } from "sonner";
import {
  getRemotePresences,
  installRemotePresence,
  removeInstalledPresence,
} from "../../../platform/pywebview/presences.api";
import type { RemotePresence } from "../../../shared/types/presence";
import Button from "../../../shared/components/Button";

export default function RemotePresencesPage() {
  const [isLoading, setIsLoading] = useState(true);
  const [presences, setPresences] = useState<RemotePresence[]>([]);
  const [searchQuery, setSearchQuery] = useState("");

  const normalize = (s: string) =>
    s
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase();

  const filteredPresences = useMemo(() => {
    const q = searchQuery.trim();
    if (!q) return presences;
    const query = normalize(q);

    return presences.filter((p) => {
      const name = normalize(p.name ?? "");
      const author = normalize(p.manifest?.author ?? "");

      return name.includes(query) || author.includes(query);
    });
  }, [presences, searchQuery]);

  const handleInstall = async (name: string) => {
    try {
      setIsLoading(true);
      const { success, message } = await installRemotePresence(name);
      toast(success ? "Installed" : "Cannot install", {
        description: message,
        duration: 5000,
      });
      if (!success) return;
      setPresences((prevPresences) =>
        prevPresences.map((p) =>
          p.name === name
            ? { ...p, manifest: { ...p.manifest, installed: true } }
            : p,
        ),
      );
    } catch (error) {
      console.error("Error installing presence:", error);
      toast("Error", {
        description: String(error),
        duration: 5000,
      });
    } finally {
      setIsLoading(false);
    }
  };

  const handleUninstall = async (name: string) => {
    try {
      let presence = presences.find((p) => p.name === name);
      if (presence && !presence.manifest.installed) {
        toast("Error", {
          description: "Presence is not installed.",
          duration: 5000,
        });
        return;
      }
      setIsLoading(true);
      const { success, message } = await removeInstalledPresence(name);
      toast(success ? "Success" : "Error", {
        description: message,
        duration: 5000,
      });
      if (!success) return;
      setPresences((prevPresences) =>
        prevPresences.map((p) =>
          p.name === name
            ? { ...p, manifest: { ...p.manifest, installed: false } }
            : p,
        ),
      );
    } catch (error) {
      console.error("Error removing presence:", error);
      toast("Error", {
        description: String(error),
        duration: 5000,
      });
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    async function fetchRemotePresences() {
      try {
        const remotePresences = await getRemotePresences();
        setPresences(remotePresences);
      } catch (error) {
        console.error("Error fetching remote presences:", error);
      } finally {
        setIsLoading(false);
      }
    }
    fetchRemotePresences();
  }, []);
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-neutral-400 text-base">Loading...</div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl">
      <div className="mb-8">
        <h1 className="text-2xl font-semibold text-white tracking-tight">
          Store
        </h1>
        <p className="text-neutral-400 mt-2 text-base">
          {presences.length} presences available
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
            placeholder="Search presences by name, description or author..."
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
          <p className="text-base text-neutral-400">No presences available</p>
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
          {filteredPresences.map((presence) => (
            <div
              key={presence.name}
              className="flex items-center justify-between p-4 rounded-xl bg-neutral-900/30 border border-neutral-800/50 hover:border-neutral-700/50"
            >
              <div className="flex items-center gap-4 min-w-0 flex-1">
                {presence.manifest.image ? (
                  <img
                    src={presence.manifest.image}
                    alt={presence.name}
                    className="w-10 h-10 rounded-lg object-cover"
                  />
                ) : (
                  <div className="w-10 h-10 rounded-lg bg-neutral-800 flex items-center justify-center">
                    <span className="text-sm font-bold text-neutral-400">
                      {presence.name.charAt(0).toUpperCase()}
                    </span>
                  </div>
                )}
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-3">
                    <p className="text-base text-neutral-200 truncate">
                      {presence.name}
                    </p>
                    {presence.manifest.installed && (
                      <span className="px-2 py-0.5 text-xs rounded bg-green-500/10 text-green-400">
                        installed
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-neutral-500 truncate">
                    {presence.manifest.description.en}
                  </p>
                </div>
              </div>

              <div className="flex items-center gap-4  ml-4">
                <div className="text-right hidden sm:block">
                  <p className="text-xs text-neutral-500">
                    v{presence.manifest.version}
                  </p>
                  <p className="text-xs text-neutral-600">
                    {presence.manifest.author}
                  </p>
                </div>
                {presence.manifest.installed ? (
                  <Button
                    onClick={() => handleUninstall(presence.name)}
                    variant="danger"
                  >
                    Uninstall
                  </Button>
                ) : (
                  <Button
                    onClick={() => handleInstall(presence.name)}
                    variant="primary"
                  >
                    Install
                  </Button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
