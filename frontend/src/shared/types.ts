import type { BrowserKind } from "./connection";

// Shapes returned by the desktop commands. Field names match the Rust side.

export type Kind = "manual" | "game" | "video" | "music" | "other";

export interface Access {
  http: string[];
  process: boolean;
  /** Options read from a program's command line, by executable name. */
  args: Record<string, string[]>;
  window: boolean;
  media: boolean;
  /** Server query protocols, such as `samp`. */
  query: string[];
  files: string[];
  sites: string[];
}

/** An option a presence declares, with the value its script receives now. */
export interface OptionView {
  type: "boolean" | "number" | "text" | "choice";
  key: string;
  label: string;
  value: boolean | number | string;
  default: boolean | number | string;
  min?: number;
  max?: number;
  max_length?: number;
  choices: string[];
  choice_labels: string[];
  /** One line under the option that says what it does, in the language in use. */
  help: string | null;
  /** The key of a boolean option that must be on, or the values other options must have. */
  when?: string | Record<string, boolean | number | string | (boolean | number | string)[]>;
}

/** Where a presence came from; only a local one can be modified. */
export type Origin = "bundled" | "catalog" | "local";

export interface PresenceView {
  name: string;
  version: string;
  description: string | null;
  kind: Kind | null;
  interval: number;
  /** The developer's choice of hiding it while paused. */
  hide_when_paused: boolean;
  enabled: boolean;
  running: boolean;
  access: Access;
  options: OptionView[];
  origin: Origin;
  editable: boolean;
  author: string | null;
  color: string | null;
  tags: string[];
  icon: string | null;
  /** Other names it is searched by. */
  other_names: string[];
  /** The hosts the person added, when the presence reads sites of one's own. */
  user_sites: string[] | null;
}

export interface Row {
  source: string;
  label: string;
  priority: number;
  kind: Kind;
  reason: string;
  selected: boolean;
}

export interface Status {
  winner: string | null;
  /** What was sent to Discord, after the privacy rules. */
  activity: Activity | null;
  rows: Row[];
  delivery_error: string | null;
  source_errors: Record<string, string>;
  /** Sources whose open pages give them nothing to read. */
  blank_sources?: string[];
}

export interface Problem {
  folder: string;
  message: string;
}

export interface Overview {
  version: string;
  discord_running: boolean;
  extensions: number;
  browsers: BrowserKind[];
  discord_user: string | null;
  paused: boolean;
  warning: string | null;
  status: Status;
  problems: Problem[];
}

export interface Pairing {
  port: number;
  token: string;
  code: string;
}

export interface GlobalPreferences {
  paused: boolean;
  privacy: "full" | "service";
  excluded_domains: string[];
  excluded_applications: string[];
  switch_delay_seconds: number;
  idle_minutes: number | null;
  paused_minutes: number;
  prefer_recent: boolean;
  /** Programs that hide everything while they run. */
  quiet_programs: string[];
  /** Hours when everything is hidden, in minutes after midnight. */
  quiet_hours: { from: number; to: number } | null;
}

export interface PresencePreferences {
  enabled: boolean;
  priority: number | null;
  show_title: boolean;
  show_buttons: boolean;
  show_timestamps: boolean;
  status_display: StatusDisplay | null;
  /** Hide while paused; null follows the presence's own choice. */
  hide_when_paused: boolean | null;
}

/** Which text the member list shows next to the user's name. */
export type StatusDisplay = "name" | "state" | "details";

export interface Preferences {
  global: GlobalPreferences;
  presences: Record<string, PresencePreferences>;
}

/** How the application starts and where minimizing sends the window. */
export interface StartupView {
  enabled: boolean;
  hidden: boolean;
  minimize_to_tray: boolean;
  asked: boolean;
}

/** The author saved in Settings and the Discord user name used while none is saved. */
export interface AuthorView {
  saved: string | null;
  discord: string | null;
}

/** The saved language (`auto` or a tag) and the language of the system. */
export interface Language {
  setting: string;
  system: string;
}

export type CatalogState = "available" | "installed" | "update" | "modified";

/** The local address for overlays: whether it is on, and its port. */
export interface LocalApiView {
  on: boolean;
  port: number;
}

/** Time on display: whether it is counted, and seconds by day and presence. */
export interface UsageView {
  on: boolean;
  days: Record<string, Record<string, number>>;
}

/** One thing a presence can reach. The interface hands it back with a yes. */
export interface Grant {
  kind: string;
  value?: unknown;
}

/** What came of asking the catalog for a presence. */
export type InstallOutcome = { state: "installed" } | { state: "asks"; access: Access; grants: Grant[] };

/** One approved presence in the catalog. */
export interface CatalogItem {
  name: string;
  version: string;
  description: string | null;
  kind: string | null;
  size: number;
  state: CatalogState;
  installed_version: string | null;
  history: string[];
  author: string | null;
  color: string | null;
  tags: string[];
  icon: string | null;
  /** Other names it is searched by. */
  other_names: string[];
  /** The catalog version needs a newer application, so it cannot be installed. */
  too_new: boolean;
}

/** The catalog page: whether it can be used, and what it lists. */
export interface CatalogView {
  available: boolean;
  items: CatalogItem[];
}

/** Whether a newer version of the application can be installed. */
export interface UpdateView {
  configured: boolean;
  version: string | null;
  notes: string | null;
}

/** The end of the newest log file. */
export interface LogView {
  folder: string;
  lines: string[];
}

export type ActivityKind = "playing" | "listening" | "watching" | "competing";

export interface ActivityButton {
  label: string;
  url: string;
}

/** What Discord shows; the same fields as the Rust `Activity`. */
export interface Activity {
  name?: string | null;
  state: string | null;
  details: string | null;
  kind: ActivityKind;
  start: number | null;
  end: number | null;
  large_image: string | null;
  large_text: string | null;
  small_image: string | null;
  small_text: string | null;
  buttons: ActivityButton[];
  party_size?: [number, number] | null;
  status_display?: StatusDisplay | null;
  details_url?: string | null;
  state_url?: string | null;
  large_url?: string | null;
  small_url?: string | null;
}

/** An activity the user wrote and saved. */
export interface ManualPreset {
  name: string;
  client_id: string;
  activity: Activity;
  show_elapsed: boolean;
}

export interface ManualView {
  presets: ManualPreset[];
  active: string | null;
  rotation: Rotation | null;
}

/** Saved statuses shown one after another. */
export interface Rotation {
  names: string[];
  seconds: number;
}

/** A saved set of publishing rules and the presences that were off. */
export interface Profile {
  name: string;
  preferences: Preferences;
  disabled: string[];
}

/** Descriptive fields of a presence that can be edited; empty text removes a field. */
export interface ManifestChanges {
  version?: string;
  description?: string;
  author?: string;
  color?: string;
  tags?: string[];
}

export type Template = "web" | "desktop";

/** What a presence says about its activity besides the activity itself. */
export interface Meta {
  foreground: boolean;
  paused: boolean;
  application: string;
  domains: string[];
}

/** What running a presence once showed. */
export interface Inspection {
  snapshot: unknown;
  activity: Activity | null;
  meta: Meta;
  error: string | null;
  console: string[];
}

/** A saved version of a presence. */
export interface VersionView {
  id: number;
  at: number;
  version: string;
  notes: string;
  files: number;
  size: number;
}

/** One file that differs from a saved version. */
export interface FileChange {
  path: string;
  change: "added" | "removed" | "changed";
  added: number;
  removed: number;
}

/** A snapshot and what the presence should show for it. */
export interface TestCase {
  name: string;
  snapshot: unknown;
  expected: unknown;
}

/** The outcome of replaying a test case. */
export interface CaseResult {
  name: string;
  passed: boolean;
  expected: unknown;
  actual: unknown;
  error: string | null;
}

/** One rule a presence must follow to enter the catalog. */
export interface ReviewCheck {
  rule: string;
  passed: boolean;
  detail: string | null;
}

/** A clean copy of a presence, ready to upload. */
export interface PreparedView {
  path: string;
  files: number;
  size: number;
}

/** What the developer tools are doing. */
export interface DevStatus {
  watching: string | null;
  folder: string | null;
  last_reload: number | null;
  error: string | null;
}

/** One connection of the browser extension. */
export interface SessionView {
  browser: BrowserKind;
  /** Digits and dots, or empty when unknown. */
  version: string;
  /** Version of the extension; empty for one from before it said so. */
  extension: string;
  /** The extension is older than the one this version of the application expects. */
  outdated: boolean;
  /** Unix seconds. */
  connected_at: number;
  /** Unix seconds; null while connected. */
  ended_at: number | null;
  /** Tabs with a page reported for a presence. */
  tabs: number;
}

/** Extensions connected now and the last ones that were. */
export interface ExtensionSessions {
  live: SessionView[];
  recent: SessionView[];
}

/** Memory and processor time of a group of processes. */
export interface Resources {
  memory_bytes: number;
  cpu_seconds: number;
  processes: number;
}

/** What the diagnostics section shows; `report` is the text to copy. */
export interface Diagnostics {
  version: string;
  resources: { app: Resources; webview: Resources } | null;
  discord_running: boolean;
  discord_connected: boolean;
  delivery_error: string | null;
  paused: boolean;
  extensions: SessionView[];
  presences: { name: string; version: string; enabled: boolean; running: boolean; error: string | null }[];
  problems: Problem[];
  report: string;
}

/** Which Discord client receives the activity when several run. */
export type DiscordClient = "auto" | "stable" | "ptb" | "canary";
