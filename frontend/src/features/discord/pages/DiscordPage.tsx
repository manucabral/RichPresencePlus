import { useEffect, useState } from "preact/hooks";
import { isDiscordRunning } from "../../../platform/pywebview/presences.api";
import InfoCard from "../../../shared/components/InfoCard";

export default function DiscordPage() {
  const [running, setRunning] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  const checkDiscord = async () => {
    setIsLoading(true);
    try {
      const result = await isDiscordRunning();
      setRunning(result);
    } catch (error) {
      console.error("Error checking Discord status:", error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    checkDiscord();
  }, []);

  return (
    <div className="max-w-2xl">
      <div className="mb-8">
        <h1 className="text-xl font-semibold text-zinc-100 tracking-tight">
          Discord
        </h1>
        <p className="text-zinc-500 mt-1 text-sm">Desktop application status</p>
      </div>

      <div className="p-6 rounded-lg bg-zinc-900/60 border border-zinc-800/60">
        <div className="flex flex-col items-center text-center">
          {isLoading ? (
            <>
              <div className="w-14 h-14 rounded-xl bg-zinc-800 flex items-center justify-center mb-4">
                <div className="w-3 h-3 rounded-full bg-zinc-600 animate-pulse" />
              </div>
              <p className="text-sm text-zinc-500">Checking status...</p>
            </>
          ) : (
            <>
              <div
                className={`w-14 h-14 rounded-xl flex items-center justify-center mb-4 ${
                  running ? "bg-emerald-500/10" : "bg-red-500/10"
                }`}
              >
                <svg
                  className={`w-7 h-7 ${running ? "text-emerald-400" : "text-red-400"}`}
                  fill="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
                </svg>
              </div>
              <div className="flex items-center gap-2 mb-1">
                <div
                  className={`w-2 h-2 rounded-full ${
                    running ? "bg-emerald-500" : "bg-red-500"
                  }`}
                />
                <span
                  className={`text-sm font-medium ${
                    running ? "text-emerald-400" : "text-red-400"
                  }`}
                >
                  {running ? "Connected" : "Not Connected"}
                </span>
              </div>
              <p className="text-xs text-zinc-600">
                {running
                  ? "Discord is currently running"
                  : "Discord is not detected on your system"}
              </p>
            </>
          )}

          <button
            onClick={checkDiscord}
            disabled={isLoading}
            className="mt-5 px-4 py-2 text-sm font-medium rounded-md bg-emerald-500/10 text-emerald-400 hover:bg-emerald-500/20 disabled:opacity-50 disabled:cursor-not-allowed border border-emerald-500/20"
          >
            {isLoading ? "Checking..." : "Refresh Status"}
          </button>
        </div>
      </div>

      <InfoCard
        text="Discord must be running to work. Make sure the Discord desktop app
            is open before enabling presences."
      />
    </div>
  );
}
