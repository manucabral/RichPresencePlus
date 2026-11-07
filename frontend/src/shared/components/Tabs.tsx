export interface Tab<K extends string> {
  key: K;
  label: string;
}

interface Props<K extends string> {
  tabs: Tab<K>[];
  active: K;
  label: string;
  onChange: (key: K) => void;
}

/** A row of tabs. The arrow keys move between them, as the pattern expects. */
export default function Tabs<K extends string>({ tabs, active, label, onChange }: Props<K>) {
  function move(event: KeyboardEvent) {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const at = tabs.findIndex((tab) => tab.key === active);
    const next = tabs[(at + step + tabs.length) % tabs.length];
    onChange(next.key);
    const list = (event.currentTarget as HTMLElement).querySelectorAll<HTMLElement>("[role=tab]");
    list[tabs.indexOf(next)]?.focus();
  }

  return (
    <div role="tablist" aria-label={label} onKeyDown={move} className="flex gap-1 overflow-x-auto overflow-y-hidden border-b border-neutral-800">
      {tabs.map((tab) => {
        const selected = tab.key === active;
        return (
          <button
            key={tab.key}
            type="button"
            role="tab"
            aria-selected={selected}
            tabIndex={selected ? 0 : -1}
            onClick={() => onChange(tab.key)}
            className={`-mb-px shrink-0 border-b-2 px-3 py-2 text-[13px] font-medium ${
              selected
                ? "border-accent text-neutral-100"
                : "border-transparent text-neutral-400 hover:text-neutral-100"
            }`}
          >
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}
