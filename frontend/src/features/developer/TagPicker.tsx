import { useT } from "../../shared/i18n";
import { MAX_TAGS, TAGS } from "../../shared/tags";

interface Props {
  chosen: string[];
  onChange: (tags: string[]) => void;
}

/** The tags a presence can carry, to turn on and off. */
export default function TagPicker({ chosen, onChange }: Props) {
  const { t } = useT();
  const full = chosen.length >= MAX_TAGS;

  return (
    <fieldset className="space-y-1.5 text-xs text-neutral-500">
      <legend>{t("developer.tags")}</legend>
      <div className="flex flex-wrap gap-1.5">
        {TAGS.map((tag) => {
          const on = chosen.includes(tag);
          return (
            <button
              key={tag}
              type="button"
              aria-pressed={on}
              disabled={full && !on}
              style={{ minHeight: 0 }}
              className={`rounded-md border px-2 py-1 text-xs transition-colors disabled:opacity-40 ${
                on
                  ? "border-accent/50 bg-accent/15 text-accent-soft"
                  : "border-neutral-800 text-neutral-400 hover:bg-white/[0.04]"
              }`}
              onClick={() => onChange(on ? chosen.filter((other) => other !== tag) : [...chosen, tag])}
            >
              {t(`tag.${tag}`)}
            </button>
          );
        })}
      </div>
    </fieldset>
  );
}
