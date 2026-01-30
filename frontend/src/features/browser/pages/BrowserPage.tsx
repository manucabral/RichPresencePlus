import { useEffect, useState } from "preact/hooks";
import {
  getInstalledBrowsers,
  getConnectedBrowser,
  launchBrowserByName,
  getCurrentCdpUrl,
  closeBrowser,
} from "../../../platform/pywebview/browser.api";
import type { Browser } from "../../../shared/types/browser";
import InfoCard from "../../../shared/components/InfoCard";

export default function BrowserPage() {
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [browsers, setBrowsers] = useState<Browser[]>([]);
  const [browser, setBrowser] = useState<Browser | null>(null);
  const [cdpUrl, setCdpUrl] = useState<string>("");

  const updateConnectedBrowser = async () => {
    const connected = await getConnectedBrowser();
    setBrowser(connected);
    if (connected) {
      const url = await getCurrentCdpUrl();
      setCdpUrl(url);
    }
  };

  const handleLaunch = async (name: string) => {
    try {
      setIsLoading(true);
      const result = await launchBrowserByName(name);
      console.log("Browser launched:", result);
      await updateConnectedBrowser();
    } catch (error) {
      console.error("Error launching browser:", error);
      alert(String(error));
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = async () => {
    try {
      if (browser) {
        setIsLoading(true);
        const result = await closeBrowser(cdpUrl);
        console.log("Browser closed:", result);
        await updateConnectedBrowser();
      }
    } catch (error) {
      console.error("Error closing browser:", error);
      alert(String(error));
      setBrowser(null);
      setCdpUrl("");
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    async function fetchData() {
      try {
        setIsLoading(true);
        const installed = await getInstalledBrowsers();
        setBrowsers(installed);
        const connected = await getConnectedBrowser();
        setBrowser(connected);
        if (connected) {
          const url = await getCurrentCdpUrl();
          setCdpUrl(url);
        }
      } catch (error) {
        console.error(error);
        alert(String(error));
      } finally {
        setIsLoading(false);
      }
    }
    fetchData();
  }, []);

  return (
    <div className="flex flex-col max-w-2xl gap-5">
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-neutral-100 tracking-tight">
          Browser
        </h1>
        <p className="text-neutral-500 mt-1 text-sm">
          Connection and configuration settings
        </p>
      </div>

      <div className="mb-6">
        <h2 className="text-xs font-medium text-neutral-500 uppercase tracking-wider mb-3">
          Connection Status
        </h2>
        <div className="p-4 rounded-xl bg-neutral-900/50 border border-neutral-800">
          {browser ? (
            <div className="space-y-4">
              <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
                <div className="flex items-center gap-3">
                  <div className="w-9 h-9 rounded-lg bg-green-500/10 flex items-center justify-center shrink-0">
                    <div className="w-2.5 h-2.5 rounded-full bg-green-500" />
                  </div>
                  <div className="min-w-0">
                    <p className="text-sm font-medium text-neutral-100 truncate">
                      {browser.name}
                    </p>
                    <p className="text-xs text-green-400">Connected</p>
                  </div>
                </div>
                <button
                  className="px-3 py-1.5 text-xs font-medium rounded-lg bg-red-500/10 text-red-400 hover:bg-red-500/20 disabled:opacity-50 shrink-0"
                  onClick={() => handleClose()}
                  disabled={isLoading}
                >
                  Disconnect
                </button>
              </div>
              <div className="pt-3 border-t border-neutral-800">
                <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
                  <span className="text-xs text-neutral-600">CDP Endpoint</span>
                  <code className="text-xs text-neutral-400 font-mono bg-neutral-800/50 px-2.5 py-1.5 rounded-lg truncate">
                    {cdpUrl}
                  </code>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex items-center gap-3">
              <div className="w-9 h-9 rounded-lg bg-neutral-800 flex items-center justify-center shrink-0">
                <div className="w-2.5 h-2.5 rounded-full bg-neutral-600" />
              </div>
              <div>
                <p className="text-sm text-neutral-400">No browser connected</p>
                <p className="text-xs text-neutral-600">
                  Select one below to connect
                </p>
              </div>
            </div>
          )}
        </div>

        <InfoCard text="If connection issues occur check network settings or close all browser instances." />
      </div>

      <div>
        <h2 className="text-xs font-medium text-neutral-500 uppercase tracking-wider mb-3">
          Available Browsers
        </h2>
        <div className="space-y-2">
          {browsers.length === 0 && !isLoading && (
            <div className="p-4 rounded-xl bg-neutral-900/30 border border-neutral-800/50 text-center">
              <p className="text-sm text-neutral-500">No browsers detected</p>
            </div>
          )}
          {browsers.map((b) => (
            <div
              key={b.name}
              className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 p-3 rounded-xl bg-neutral-900/30 border border-neutral-800/50 hover:border-neutral-700/50"
            >
              <div className="flex items-center gap-3 min-w-0">
                <div className="w-8 h-8 rounded-lg bg-neutral-800 flex items-center justify-center shrink-0">
                  <span className="text-xs font-semibold text-neutral-500">
                    {b.name.charAt(0)}
                  </span>
                </div>
                <div className="min-w-0">
                  <p className="text-sm text-neutral-200 truncate">{b.name}</p>
                  <p className="text-xs text-neutral-600 font-mono truncate">
                    {b.path}
                  </p>
                </div>
              </div>
              <button
                className="px-3 py-1.5 text-xs font-medium rounded-lg bg-green-500/10 text-green-400 hover:bg-green-500/20 disabled:opacity-50 shrink-0"
                onClick={() => handleLaunch(b.name)}
                disabled={isLoading}
              >
                Connect
              </button>
            </div>
          ))}
        </div>
        <InfoCard text="If browsers are not listed please report an issue." />
      </div>
    </div>
  );
}
