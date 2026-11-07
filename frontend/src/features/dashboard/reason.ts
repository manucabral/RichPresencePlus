/**
 * The selection policy explains each row in English (`rpp-core/src/policy.rs`).
 * This maps those sentences to translation keys; a sentence it does not know
 * is shown as the policy wrote it.
 */
const EXACT: Record<string, string> = {
  "Publishing is paused": "reason.paused",
  "Disabled for this presence": "reason.disabled",
  "Application excluded by privacy preferences": "reason.application",
  "Domain excluded by privacy preferences": "reason.domain",
  "Media is paused": "reason.mediaPaused",
  "Presence stopped reporting": "reason.stopped",
  "Waiting before switching": "reason.waiting",
  "Another activity has a higher priority; on a tie the current one stays": "reason.priority",
  "Hidden while the computer is idle": "reason.idle",
  "Hidden by your automatic rules": "reason.quiet",
};

type Translate = (key: string, values?: Record<string, string | number>) => string;

/** Translates the policy's reason for a row into the language in use. */
export function reasonText(reason: string, t: Translate): string {
  const key = EXACT[reason];
  if (key) return t(key);
  const selected = /^Selected: (\w+), priority (\d+)$/.exec(reason);
  if (selected) {
    return t("reason.selected", { kind: t(`kind.${selected[1].toLowerCase()}`), priority: selected[2] });
  }
  return reason;
}

/** The text of a presence's problem; only known ones have a translation. */
export function problemText(message: string, t: (key: string) => string): string {
  return message === "This presence needs a valid Discord application ID" ? t("problem.noApplication") : message;
}

/**
 * The text of a failed delivery. Only a program in Discord's place has its own
 * wording: the rest comes from the system and is shown as it is.
 */
export function deliveryText(message: string, t: Translate): string {
  const taken = /^(.+) holds the connection to Discord$/.exec(message);
  return taken ? t("dashboard.pipeTaken", { name: taken[1] }) : t("dashboard.discordError", { message });
}
