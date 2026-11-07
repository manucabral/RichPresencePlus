import type { Tone } from "../../features/dashboard/tone";

/** Colors of each tone; the shapes below carry the meaning without color too. */
const COLOR: Record<Tone, string> = {
  showing: "text-accent",
  waiting: "text-warning",
  paused: "text-neutral-400",
  standby: "text-neutral-500",
  hidden: "text-neutral-600",
  stopped: "text-danger",
};

/** A 14 px state icon: full, half, pause, empty, dashed or crossed circle. */
export default function StatusIcon({ tone, label }: { tone: Tone; label: string }) {
  return (
    <svg viewBox="0 0 14 14" className={`h-3.5 w-3.5 shrink-0 ${COLOR[tone]}`} role="img" aria-label={label}>
      <circle
        cx="7"
        cy="7"
        r="6"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeDasharray={tone === "hidden" ? "2 2" : undefined}
      />
      {tone === "showing" && <circle cx="7" cy="7" r="3.5" fill="currentColor" />}
      {tone === "waiting" && <path d="M7 3.5a3.5 3.5 0 0 1 0 7z" fill="currentColor" />}
      {tone === "paused" && <path d="M5.5 4.5v5M8.5 4.5v5" stroke="currentColor" strokeWidth="1.5" />}
      {tone === "stopped" && <path d="M5 5l4 4M9 5l-4 4" stroke="currentColor" strokeWidth="1.5" />}
    </svg>
  );
}

/** One to three bars, like a signal meter, for a priority. */
export function PriorityBars({ filled, label }: { filled: 1 | 2 | 3; label: string }) {
  return (
    <svg viewBox="0 0 14 14" className="h-3.5 w-3.5 shrink-0" role="img" aria-label={label}>
      {[0, 1, 2].map((bar) => (
        <rect
          key={bar}
          x={1.5 + bar * 4}
          y={9 - bar * 3}
          width="3"
          height={4 + bar * 3}
          rx="0.75"
          className={bar < filled ? "fill-neutral-300" : "fill-neutral-700"}
        />
      ))}
    </svg>
  );
}
