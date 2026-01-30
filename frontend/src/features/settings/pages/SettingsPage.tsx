import { useEffect, useState } from "preact/hooks";
import {
  getUserSettingKey,
  setUserSettingKey,
} from "../../../platform/pywebview/user.api";

const LOG_LEVELS = ["DEBUG", "INFO", "WARNING", "ERROR"] as const;
type LogLevel = (typeof LOG_LEVELS)[number];

export default function SettingsPage() {
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">(
    "idle",
  );

  const [profileName, setProfileName] = useState<string>("");
  const [runtimeInterval, setRuntimeInterval] = useState<string>("5");
  const [browserTargetPort, setBrowserTargetPort] = useState<string>("9222");
  const [logsLevel, setLogsLevel] = useState<LogLevel>("INFO");

  const [portError, setPortError] = useState<string>("");
  const [intervalError, setIntervalError] = useState<string>("");

  const validatePort = (value: string): boolean => {
    if (!value) {
      setPortError("Port is required");
      return false;
    }
    const port = parseInt(value, 10);
    if (isNaN(port)) {
      setPortError("Must be a number");
      return false;
    }
    if (port < 1 || port > 65535) {
      setPortError("Must be between 1-65535");
      return false;
    }
    setPortError("");
    return true;
  };

  const validateInterval = (value: string): boolean => {
    if (!value) {
      setIntervalError("Interval is required");
      return false;
    }
    const interval = parseInt(value, 10);
    if (isNaN(interval)) {
      setIntervalError("Must be a number");
      return false;
    }
    if (interval < 1 || interval > 300) {
      setIntervalError("Must be between 1-300");
      return false;
    }
    setIntervalError("");
    return true;
  };

  const handlePortChange = (value: string) => {
    setBrowserTargetPort(value);
    validatePort(value);
  };

  const handleIntervalChange = (value: string) => {
    setRuntimeInterval(value);
    validateInterval(value);
  };

  const loadSettings = async () => {
    try {
      setIsLoading(true);
      const [profile, interval, port, logs] = await Promise.all([
        getUserSettingKey("profile_name"),
        getUserSettingKey("runtime_interval"),
        getUserSettingKey("browser_target_port"),
        getUserSettingKey("logs_level"),
      ]);

      setProfileName(profile || "");
      setRuntimeInterval(String(interval || 5));
      setBrowserTargetPort(String(port || 9222));
      setLogsLevel((logs as LogLevel) || "INFO");
    } catch (error) {
      console.error("Error loading settings:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const saveSettings = async () => {
    const isPortValid = validatePort(browserTargetPort);
    const isIntervalValid = validateInterval(runtimeInterval);

    if (!isPortValid || !isIntervalValid) {
      return;
    }

    try {
      setIsSaving(true);
      setSaveStatus("idle");

      await Promise.all([
        setUserSettingKey("profile_name", profileName),
        setUserSettingKey("runtime_interval", parseInt(runtimeInterval, 10)),
        setUserSettingKey(
          "browser_target_port",
          parseInt(browserTargetPort, 10),
        ),
        setUserSettingKey("logs_level", logsLevel),
      ]);

      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (error) {
      console.error("Error saving settings:", error);
      setSaveStatus("error");
      setTimeout(() => setSaveStatus("idle"), 3000);
    } finally {
      setIsSaving(false);
    }
  };

  useEffect(() => {
    loadSettings();
  }, []);

  return (
    <div className="flex flex-col max-w-2xl gap-6">
      <div>
        <h1 className="text-xl font-semibold text-neutral-100 tracking-tight">
          Settings
        </h1>
        <p className="text-neutral-500 mt-1 text-sm">
          Configure application preferences
        </p>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <svg
            className="w-5 h-5 text-green-500 animate-spin"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            />
          </svg>
        </div>
      ) : (
        <>
          <div>
            <h2 className="text-xs font-medium text-neutral-500 uppercase tracking-wider mb-3">
              Profile
            </h2>
            <div className="p-4 rounded-xl bg-neutral-900/50 border border-neutral-800">
              <div>
                <label className="block text-sm font-medium text-neutral-300 mb-1.5">
                  Profile Name
                </label>
                <input
                  type="text"
                  value={profileName}
                  onChange={(e) =>
                    setProfileName((e.target as HTMLInputElement).value)
                  }
                  placeholder="My Profile"
                  className="w-full px-3 py-2 text-sm bg-neutral-800/50 border border-neutral-700/50 rounded-lg text-neutral-100 placeholder-neutral-600 focus:outline-none focus:border-green-500/50 focus:ring-1 focus:ring-green-500/20"
                />
                <p className="text-[11px] text-neutral-600 mt-1.5">
                  Name to identify this configuration profile
                </p>
              </div>
            </div>
          </div>

          <div>
            <h2 className="text-xs font-medium text-neutral-500 uppercase tracking-wider mb-3">
              Connection
            </h2>
            <div className="flex flex-col gap-4 p-4 rounded-xl bg-neutral-900/50 border border-neutral-800">
              <div>
                <label className="block text-sm font-medium text-neutral-300 mb-1.5">
                  Browser Target Port
                </label>
                <input
                  type="number"
                  value={browserTargetPort}
                  onChange={(e) =>
                    handlePortChange((e.target as HTMLInputElement).value)
                  }
                  placeholder="9222"
                  min="1"
                  max="65535"
                  className={`w-full px-3 py-2 text-sm bg-neutral-800/50 border rounded-lg text-neutral-100 placeholder-neutral-600 focus:outline-none focus:ring-1 ${
                    portError
                      ? "border-red-500/50 focus:border-red-500/50 focus:ring-red-500/20"
                      : "border-neutral-700/50 focus:border-green-500/50 focus:ring-green-500/20"
                  }`}
                />
                {portError ? (
                  <p className="text-[11px] text-red-400 mt-1.5">{portError}</p>
                ) : (
                  <p className="text-[11px] text-neutral-600 mt-1.5">
                    CDP debugging port for browser connection (1-65535)
                  </p>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-neutral-300 mb-1.5">
                  Update Interval
                </label>
                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    value={runtimeInterval}
                    onChange={(e) =>
                      handleIntervalChange((e.target as HTMLInputElement).value)
                    }
                    placeholder="5"
                    min="1"
                    max="300"
                    className={`flex-1 px-3 py-2 text-sm bg-neutral-800/50 border rounded-lg text-neutral-100 placeholder-neutral-600 focus:outline-none focus:ring-1 ${
                      intervalError
                        ? "border-red-500/50 focus:border-red-500/50 focus:ring-red-500/20"
                        : "border-neutral-700/50 focus:border-green-500/50 focus:ring-green-500/20"
                    }`}
                  />
                  <span className="text-sm text-neutral-500 shrink-0">
                    seconds
                  </span>
                </div>
                {intervalError ? (
                  <p className="text-[11px] text-red-400 mt-1.5">
                    {intervalError}
                  </p>
                ) : (
                  <p className="text-[11px] text-neutral-600 mt-1.5">
                    How often to refresh presence data (1-300 seconds)
                  </p>
                )}
              </div>
            </div>
          </div>

          <div>
            <h2 className="text-xs font-medium text-neutral-500 uppercase tracking-wider mb-3">
              Logging
            </h2>
            <div className="p-4 rounded-xl bg-neutral-900/50 border border-neutral-800">
              <div>
                <label className="block text-sm font-medium text-neutral-300 mb-1.5">
                  Log Level
                </label>
                <select
                  value={logsLevel}
                  onChange={(e) =>
                    setLogsLevel(
                      (e.target as HTMLSelectElement).value as LogLevel,
                    )
                  }
                  className="w-full px-3 py-2 text-sm bg-neutral-800/50 border border-neutral-700/50 rounded-lg text-neutral-100 focus:outline-none focus:border-green-500/50 focus:ring-1 focus:ring-green-500/20 appearance-none cursor-pointer"
                >
                  {LOG_LEVELS.map((level) => (
                    <option
                      key={level}
                      value={level}
                      className="bg-neutral-800 text-neutral-100"
                    >
                      {level}
                    </option>
                  ))}
                </select>
                <p className="text-[11px] text-neutral-600 mt-1.5">
                  Controls the verbosity of application logs
                </p>
              </div>
            </div>
          </div>

          <div className="flex items-center justify-between pt-2">
            <div className="text-sm">
              {saveStatus === "success" && (
                <span className="text-green-400 flex items-center gap-1.5">
                  <svg
                    className="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Settings saved
                </span>
              )}
              {saveStatus === "error" && (
                <span className="text-red-400 flex items-center gap-1.5">
                  <svg
                    className="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                  Failed to save
                </span>
              )}
            </div>
            <button
              onClick={saveSettings}
              disabled={isSaving || !!portError || !!intervalError}
              className="flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-lg bg-green-500/10 text-green-400 hover:bg-green-500/20 disabled:opacity-50 disabled:cursor-not-allowed border border-green-500/20"
            >
              {isSaving ? (
                <>
                  <svg
                    className="w-4 h-4 animate-spin"
                    fill="none"
                    viewBox="0 0 24 24"
                  >
                    <circle
                      className="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      strokeWidth="4"
                    />
                    <path
                      className="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    />
                  </svg>
                  Saving...
                </>
              ) : (
                <>
                  <svg
                    className="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={1.5}
                      d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"
                    />
                  </svg>
                  Save Settings
                </>
              )}
            </button>
          </div>
        </>
      )}
    </div>
  );
}
