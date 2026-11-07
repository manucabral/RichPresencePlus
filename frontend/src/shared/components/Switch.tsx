interface SwitchProps {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
  disabled?: boolean;
}

export default function Switch({ checked, onChange, label, disabled = false }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative h-5 w-9 shrink-0 rounded-full border transition-colors disabled:opacity-50 ${
        checked ? "bg-accent border-accent" : "bg-neutral-800 border-neutral-700"
      }`}
      style={{ minHeight: 0 }}
    >
      <span
        className={`absolute top-px h-4 w-4 rounded-full shadow transition-all ${
          checked ? "left-4 bg-white" : "left-0.5 bg-neutral-400"
        }`}
      />
    </button>
  );
}
