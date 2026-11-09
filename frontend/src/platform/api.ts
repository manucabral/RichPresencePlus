// The only place that talks to the desktop side.
import { invoke } from "@tauri-apps/api/core";
import type {
  StartupView,
  AuthorView,
  CatalogView,
  Grant,
  InstallOutcome,
  LocalApiView,
  UsageView,
  DevStatus,
  CaseResult,
  FileChange,
  Inspection,
  TestCase,
  VersionView,
  Language,
  LogView,
  ManifestChanges,
  ManualPreset,
  ManualView,
  Overview,
  Pairing,
  Preferences,
  PresenceView,
  Problem,
  Profile,
  Template,
  UpdateView,
  ExtensionSessions,
  PreparedView,
  ReviewCheck,
  Diagnostics,
  DiscordClient,
} from "../shared/types";

export const getOverview = () => invoke<Overview>("overview");
export const getPresences = () => invoke<PresenceView[]>("presences");
export const setPresenceEnabled = (name: string, enabled: boolean) =>
  invoke<void>("set_presence_enabled", { name, enabled });
export const setPresenceOption = (name: string, key: string, value: boolean | number | string) =>
  invoke<void>("set_presence_option", { name, key, value });
export const setPresenceSites = (name: string, sites: string[]) =>
  invoke<string[]>("set_presence_sites", { name, sites });
export const getIcons = () => invoke<Record<string, string>>("presence_icons");
export const updateManifest = (name: string, changes: ManifestChanges) =>
  invoke<void>("update_manifest", { name, changes });
export const getPreferences = () => invoke<Preferences>("preferences");
export const savePreferences = (preferences: Preferences) =>
  invoke<Preferences>("save_preferences", { preferences });
export const getPairing = () => invoke<Pairing>("pairing");
export const getLanguage = () => invoke<Language>("language");
export const setLanguage = (tag: string) => invoke<void>("set_language", { tag });
export const getCatalog = (refresh = false) => invoke<CatalogView>("catalog", { refresh });
export const getLocalApi = () => invoke<LocalApiView>("local_api");
export const setLocalApi = (on: boolean) => invoke<void>("set_local_api", { on });
export const getUsage = () => invoke<UsageView>("usage");
export const setTrackTime = (on: boolean) => invoke<void>("set_track_time", { on });
export const exportUsage = () => invoke<string>("export_usage");
export const clearUsage = () => invoke<void>("clear_usage");
export const installPresence = (name: string, accepted: Grant[] = []) =>
  invoke<InstallOutcome>("install_presence", { name, accepted });
export const removePresence = (name: string) => invoke<void>("remove_presence", { name });
export const restorePresence = (name: string, version: string) =>
  invoke<void>("restore_presence", { name, version });
export const checkUpdate = () => invoke<UpdateView>("check_update");

let updateCheck: Promise<UpdateView> | null = null;
/** Asks for an update once per session; after a failure the next call asks again. */
export const checkUpdateOnce = (): Promise<UpdateView> =>
  (updateCheck ??= checkUpdate().catch((error: unknown) => {
    updateCheck = null;
    throw error;
  }));
export const getExtensionSessions = () => invoke<ExtensionSessions>("extension_sessions");
export const getFavorites = () => invoke<string[]>("favorites");
export const getDiagnostics = () => invoke<Diagnostics>("diagnostics");
export const getDiscordClient = () => invoke<DiscordClient>("discord_client");
export const getPauseShortcut = () => invoke<string | null>("pause_shortcut");
export const setPauseShortcut = (shortcut: string | null) => invoke<string | null>("set_pause_shortcut", { shortcut });
export const setDiscordClient = (choice: DiscordClient) => invoke<void>("set_discord_client", { choice });
export const exportManualPreset = (name: string) => invoke<string>("export_manual_preset", { name });
export const startRotation = (names: string[], seconds: number) => invoke<void>("start_rotation", { names, seconds });
export const stopRotation = () => invoke<void>("stop_rotation");
export const importManualPreset = (text: string) => invoke<ManualPreset>("import_manual_preset", { text });
export const setFavorite = (name: string, favorite: boolean) => invoke<void>("set_favorite", { name, favorite });
export const installUpdate = () => invoke<void>("install_update");
export const getAuthor = () => invoke<AuthorView>("author");
export const setAuthor = (author: string) => invoke<void>("set_author", { author });
export const getStartup = () => invoke<StartupView>("startup");
export const saveStartup = (enabled: boolean, hidden: boolean, minimizeToTray: boolean) =>
  invoke<void>("save_startup", { enabled, hidden, minimizeToTray });
export const getWindowVisible = () => invoke<boolean>("window_visible");
export const getLogLevel = () => invoke<string>("log_level");
export const setLogLevel = (level: string) => invoke<void>("set_log_level", { level });
export const getLogs = () => invoke<LogView>("logs");
export const getManual = () => invoke<ManualView>("manual");
export const saveManualPreset = (preset: ManualPreset) => invoke<void>("save_manual_preset", { preset });
export const deleteManualPreset = (name: string) => invoke<void>("delete_manual_preset", { name });
export const showManual = (name: string | null) => invoke<void>("show_manual", { name });
export const getProfiles = () => invoke<Profile[]>("profiles");
export const saveProfile = (name: string) => invoke<void>("save_profile", { name });
export const deleteProfile = (name: string) => invoke<void>("delete_profile", { name });
export const applyProfile = (name: string) => invoke<void>("apply_profile", { name });
export const createPresence = (name: string, template: Template) =>
  invoke<string>("create_presence", { name, template });
export const deletePresence = (name: string) => invoke<void>("delete_presence", { name });
export const getRunningPrograms = () => invoke<string[]>("running_programs");
export const createProgramPresence = (choice: { name: string; program: string; details: string; state: string }) =>
  invoke<string>("create_program_presence", { choice });
export const copyPresence = (name: string, newName: string) =>
  invoke<string>("copy_presence", { name, newName });
export const reloadPresences = () => invoke<Problem[]>("reload_presences");
export const inspectPresence = (name: string) => invoke<Inspection>("inspect_presence", { name });
export const watchPresence = (name: string | null) => invoke<void>("watch_presence", { name });
export const getVersions = (name: string) => invoke<VersionView[]>("presence_versions", { name });
export const saveVersion = (name: string, notes: string) => invoke<VersionView>("save_version", { name, notes });
export const compareVersion = (name: string, id: number) => invoke<FileChange[]>("compare_version", { name, id });
export const restoreVersion = (name: string, id: number) => invoke<void>("restore_version", { name, id });
export const getTestCases = (name: string) => invoke<TestCase[]>("test_cases", { name });
export const saveTestCase = (name: string, testCase: TestCase) =>
  invoke<void>("save_test_case", { name, case: testCase });
export const deleteTestCase = (name: string, testCase: string) =>
  invoke<void>("delete_test_case", { name, case: testCase });
export const runTestCases = (name: string) => invoke<CaseResult[]>("run_test_cases", { name });
export const reviewPresence = (name: string) => invoke<ReviewCheck[]>("review_presence", { name });
export const preparePresence = (name: string) => invoke<PreparedView>("prepare_presence", { name });
export const hasCodeEditor = () => invoke<boolean>("has_code_editor");
export const openInCodeEditor = (name: string) => invoke<void>("open_in_code_editor", { name });
export const getDevStatus = () => invoke<DevStatus>("dev_status");
export const openLink = (target: string) => invoke<void>("open_link", { target });
export const openLegal = (page: string, language: string) => invoke<void>("open_legal", { page, language });
export const openPresenceFolder = (name: string) => invoke<void>("open_presence_folder", { name });
