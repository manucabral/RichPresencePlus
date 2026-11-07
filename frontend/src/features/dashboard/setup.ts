/** The steps of the guide on the first screen. Pure, so it is tested without a window. */

export interface SetupInput {
  discordRunning: boolean;
  /** Browsers connected through the extension. */
  extensions: number;
  /** Presences turned on. */
  enabled: number;
  /** Presences offering an activity right now. */
  offering: number;
}

export interface SetupStep {
  /** Names its texts: `setup.<id>` and `setup.<id>Hint`. */
  id: "discord" | "browser" | "presence";
  done: boolean;
  /** The page that solves it, if the app has one. */
  link: string | null;
}

/** What is left to see a first activity on Discord, in the order to do it. */
export function setupSteps(input: SetupInput): SetupStep[] {
  return [
    { id: "discord", done: input.discordRunning, link: null },
    { id: "browser", done: input.extensions > 0, link: "/browser" },
    { id: "presence", done: input.enabled > 0 && input.offering > 0, link: "/presences" },
  ];
}

/** The first step still to do; the guide marks it. */
export function nextStep(steps: SetupStep[]): SetupStep["id"] | null {
  return steps.find((step) => !step.done)?.id ?? null;
}
