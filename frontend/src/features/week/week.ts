/** Seconds on display, by presence, in one day. */
export type Day = Record<string, number>;

/** One day of the chart. */
export interface WeekDay {
  /** The day, as `YYYY-MM-DD`. */
  date: string;
  /** Seconds on display. */
  seconds: number;
}

/** A presence and its time in the week. */
export interface WeekName {
  name: string;
  seconds: number;
}

/** A date on the local clock as `YYYY-MM-DD`, the way the host keys its days. */
export function dayKey(date: Date): string {
  const part = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${part(date.getMonth() + 1)}-${part(date.getDate())}`;
}

/** The seven days that end today, oldest first; a day with nothing counted has zero. */
export function lastWeek(today: Date, days: Record<string, Day>): WeekDay[] {
  const week: WeekDay[] = [];
  for (let back = 6; back >= 0; back -= 1) {
    // Moved by calendar day, so a day with a clock change still counts as one.
    const date = dayKey(new Date(today.getFullYear(), today.getMonth(), today.getDate() - back));
    const seconds = Object.values(days[date] ?? {}).reduce((sum, value) => sum + value, 0);
    week.push({ date, seconds });
  }
  return week;
}

/** Adds up each presence over the seven days, most time first. */
export function namesOfTheWeek(today: Date, days: Record<string, Day>): WeekName[] {
  const totals = new Map<string, number>();
  for (const { date } of lastWeek(today, days)) {
    for (const [name, seconds] of Object.entries(days[date] ?? {})) {
      totals.set(name, (totals.get(name) ?? 0) + seconds);
    }
  }
  return [...totals]
    .map(([name, seconds]) => ({ name, seconds }))
    .sort((a, b) => b.seconds - a.seconds || a.name.localeCompare(b.name));
}

/** Splits seconds into whole hours and minutes. */
export function hoursAndMinutes(seconds: number): { hours: number; minutes: number } {
  const total = Math.floor(seconds / 60);
  return { hours: Math.floor(total / 60), minutes: total % 60 };
}
