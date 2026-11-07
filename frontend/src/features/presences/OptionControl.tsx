import type { OptionView } from "../../shared/types";

const field = "rounded-md border border-neutral-800 bg-neutral-950 px-2 py-1 text-[13px] text-neutral-100";

interface Props {
  option: OptionView;
  onSave: (value: boolean | number | string) => void;
}

/**
 * One option a presence declares, drawn as the control its type calls for.
 * Inputs are uncontrolled: the list refreshes every few seconds and would
 * overwrite what the user types.
 */
export default function OptionControl({ option, onSave }: Props) {
  const key = `${option.key}:${String(option.value)}`;

  if (option.type === "boolean") {
    return (
      <label className="flex items-center gap-2 text-[13px] text-neutral-300">
        <input
          type="checkbox"
          checked={option.value === true}
          onChange={(event) => onSave((event.target as HTMLInputElement).checked)}
        />
        {option.label}
      </label>
    );
  }

  if (option.type === "choice") {
    return (
      <label className="flex items-center justify-between gap-3 text-[13px] text-neutral-300">
        {option.label}
        <select
          key={key}
          defaultValue={String(option.value)}
          className={field}
          onChange={(event) => onSave((event.target as HTMLSelectElement).value)}
        >
          {option.choices.map((choice, index) => (
            <option key={choice} value={choice}>
              {option.choice_labels[index] ?? choice}
            </option>
          ))}
        </select>
      </label>
    );
  }

  if (option.type === "number") {
    return (
      <label className="flex items-center justify-between gap-3 text-[13px] text-neutral-300">
        {option.label}
        <input
          key={key}
          type="number"
          min={option.min ?? undefined}
          max={option.max ?? undefined}
          defaultValue={String(option.value)}
          className={`w-24 ${field}`}
          onChange={(event) => {
            const number = Number((event.target as HTMLInputElement).value);
            if (Number.isFinite(number)) onSave(number);
          }}
        />
      </label>
    );
  }

  return (
    <label className="flex items-center justify-between gap-3 text-[13px] text-neutral-300">
      {option.label}
      <input
        key={key}
        type="text"
        maxLength={option.max_length ?? 64}
        defaultValue={String(option.value)}
        className={`w-48 ${field}`}
        onChange={(event) => onSave((event.target as HTMLInputElement).value)}
      />
    </label>
  );
}
