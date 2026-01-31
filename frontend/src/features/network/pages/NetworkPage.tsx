import { toast } from "sonner";
import { useEffect, useState } from "preact/hooks";
import {
  getNetworkProcceses,
  closeNetworkProcesses,
} from "../../../platform/pywebview/network.api";
import type { NetworkProcess } from "../../../shared/types/networkprocess";
import Button from "../../../shared/components/Button";

export default function NetworkPage() {
  const [processesRaw, setProcessesRaw] = useState<
    Record<number, NetworkProcess[]>
  >({});

  useEffect(() => {
    async function fetchProcesses() {
      try {
        const networkProcesses = await getNetworkProcceses();
        console.log("Network processes:", networkProcesses);
        setProcessesRaw(networkProcesses);
      } catch (error) {
        console.error("Error fetching network processes:", error);
      }
    }
    fetchProcesses();
  }, []);

  const handleCloseProcesses = async (port: number) => {
    try {
      const result = await closeNetworkProcesses(port);
      console.log(`Closed processes on port ${port}:`, result);
      const updatedProcesses = await getNetworkProcceses();
      setProcessesRaw(updatedProcesses);
    } catch (error) {
      console.error(`Error closing processes on port ${port}:`, error);
      toast("Error", {
        description: "Failed to close processes.",
        duration: 5000,
      });
    }
  };

  const getKeys = (obj: any): string[] => {
    return obj ? Object.keys(obj) : [];
  };

  const availablePorts = getKeys(processesRaw);

  return (
    <div className="max-w-4xl">
      <div className="mb-8">
        <h1 className="text-2xl font-semibold text-white tracking-tight">
          Network
        </h1>
        <p className="text-neutral-400 mt-2 text-base">Active processes</p>
      </div>

      {availablePorts.length === 0 ? (
        <div className="p-8 rounded-xl bg-neutral-900/50 border border-neutral-800 text-center">
          <div className="w-12 h-12 rounded-lg bg-neutral-800 flex items-center justify-center mx-auto mb-4">
            <div className="w-3 h-3 rounded-full bg-neutral-600" />
          </div>
          <p className="text-base text-neutral-400">No active processes</p>
        </div>
      ) : (
        <div className="space-y-4">
          {availablePorts.map((port) => {
            const entries = processesRaw[Number(port)] ?? [];
            return (
              <div
                key={port}
                className="p-5 rounded-xl bg-neutral-900/50 border border-neutral-800"
              >
                <div className="flex items-center justify-between mb-4">
                  <div className="flex items-center gap-3">
                    <code className="text-sm font-mono text-green-400 bg-green-500/10 px-3 py-1 rounded-lg">
                      :{port}
                    </code>
                    <span className="text-sm text-neutral-500">
                      {entries.length} process{entries.length !== 1 ? "es" : ""}
                    </span>
                  </div>
                  {entries.length > 0 && (
                    <Button
                      onClick={() => handleCloseProcesses(Number(port))}
                      variant="danger"
                      className="text-sm"
                    >
                      Kill all
                    </Button>
                  )}
                </div>
                {entries.length > 0 && (
                  <div className="space-y-2">
                    {entries.map((process, i) => (
                      <div
                        key={`${port}-${process.pid}-${i}`}
                        className="flex items-center justify-between py-2.5 px-4 rounded-lg bg-neutral-800/30 text-sm"
                      >
                        <code className="text-neutral-300 font-mono">
                          PID {process.pid}
                        </code>
                        <span
                          className={
                            process.state === "LISTEN"
                              ? "text-green-400"
                              : "text-neutral-500"
                          }
                        >
                          {process.state}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
