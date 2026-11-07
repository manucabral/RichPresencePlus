// Building blocks every page shares, so spacing, borders and type stay the same.
import type { ComponentChildren } from "preact";

/**
 * Width of every page's column, centered in the main area. One width keeps the
 * title in the same place from page to page. It stays the same on large
 * screens: a row stretched to the whole window puts its parts too far apart.
 */
export const pageWidth = "mx-auto w-full max-w-3xl";

/** A one-line text box or number box. */
export const fieldClass =
  "w-full min-w-0 rounded-md border border-neutral-800 bg-neutral-950 px-3 py-1.5 text-[13px] text-neutral-100 placeholder:text-neutral-600";

/** A drop-down list next to a setting. */
export const selectClass =
  "max-w-full rounded-md border border-neutral-800 bg-neutral-900 px-2 py-1 text-[13px] text-neutral-100";

/** A box for several lines, such as a list of sites. */
export const areaClass =
  "w-full rounded-md border border-neutral-800 bg-neutral-950 px-3 py-2 text-[13px] text-neutral-100 placeholder:text-neutral-600";

/** A titled group of content, with an optional action on the right. */
export function Section({
  title,
  hint,
  action,
  children,
}: {
  title: string;
  hint?: string;
  action?: ComponentChildren;
  children: ComponentChildren;
}) {
  return (
    <section className="mb-8">
      <div className="mb-3 flex items-end justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-[13px] font-medium text-neutral-400">{title}</h2>
          {hint && <p className="mt-0.5 text-xs text-neutral-600">{hint}</p>}
        </div>
        {action}
      </div>
      {children}
    </section>
  );
}

/** A bordered box whose children are separated by thin lines. */
export function Panel({ children, className = "" }: { children: ComponentChildren; className?: string }) {
  return (
    <div className={`divide-y divide-neutral-800/70 rounded-lg border border-neutral-800 bg-neutral-900/40 ${className}`}>
      {children}
    </div>
  );
}

/** Rows separated by thin lines, for use inside something that already has a border. */
export function Rows({ children }: { children: ComponentChildren }) {
  return <div className="divide-y divide-neutral-800/70">{children}</div>;
}

/** A setting inside a panel: what it is on the left, its control on the right. */
export function SettingRow({
  title,
  hint,
  children,
  stacked = false,
}: {
  title: string;
  hint?: string;
  children: ComponentChildren;
  /** Puts the control under the text, for wide controls such as text boxes. */
  stacked?: boolean;
}) {
  return (
    // Side by side while the text keeps about 12rem; below that the control
    // wraps under it instead of squeezing the text into a narrow column.
    <div className={`px-4 py-3 ${stacked ? "space-y-2" : "flex flex-wrap items-center justify-between gap-x-4 gap-y-2"}`}>
      <div className={stacked ? "min-w-0" : "min-w-0 flex-1 basis-48"}>
        <p className="text-[13px] text-neutral-100">{title}</p>
        {hint && <p className="mt-0.5 text-xs text-neutral-500">{hint}</p>}
      </div>
      <div className={stacked ? "" : "max-w-full shrink-0"}>{children}</div>
    </div>
  );
}

/** A small rounded label, such as a kind or an origin. */
export function Chip({ children, tone = "neutral", title }: { children: ComponentChildren; tone?: "neutral" | "accent"; title?: string }) {
  const colors = tone === "accent" ? "bg-accent/15 text-accent-soft" : "bg-neutral-800 text-neutral-400";
  return (
    <span title={title} className={`rounded px-1.5 py-px text-[11px] ${colors}`}>
      {children}
    </span>
  );
}
