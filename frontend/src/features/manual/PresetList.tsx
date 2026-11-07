import Button from "../../shared/components/Button";
import { useT } from "../../shared/i18n";
import type { ManualPreset } from "../../shared/types";

interface Props {
  presets: ManualPreset[];
  active: string | null;
  busy: boolean;
  onShow: (name: string | null) => void;
  onEdit: (preset: ManualPreset) => void;
  onDelete: (name: string) => void;
  onCopy: (name: string) => void;
}

export default function PresetList({ presets, active, busy, onShow, onEdit, onDelete, onCopy }: Props) {
  const { t } = useT();
  if (presets.length === 0) {
    return <p className="text-sm text-neutral-500">{t("manual.none")}</p>;
  }
  return (
    <ul className="space-y-2">
      {presets.map((preset) => {
        const on = preset.name === active;
        return (
          <li
            key={preset.name}
            className={`flex items-center justify-between gap-3 rounded-lg border px-4 py-3 ${
              on ? "border-accent/30 bg-accent/5" : "border-neutral-800 bg-neutral-900/40"
            }`}
          >
            <div className="min-w-0">
              <p className="truncate text-sm text-neutral-100">{preset.name}</p>
              <p className="truncate text-xs text-neutral-500">
                {on ? t("manual.onDisplay") : (preset.activity.details ?? preset.activity.state ?? "")}
              </p>
            </div>
            <div className="flex shrink-0 gap-1">
              <Button
                variant={on ? "warning" : "primary"}
                className="text-xs"
                disabled={busy}
                onClick={() => onShow(on ? null : preset.name)}
              >
                {on ? t("manual.stop") : t("manual.show")}
              </Button>
              <Button variant="ghost" className="text-xs" disabled={busy} onClick={() => onEdit(preset)}>
                {t("manual.edit")}
              </Button>
              <Button
                variant="ghost"
                className="text-xs"
                disabled={busy}
                title={t("manual.copyHint")}
                onClick={() => onCopy(preset.name)}
              >
                {t("manual.copy")}
              </Button>
              <Button variant="ghost" className="text-xs" disabled={busy} onClick={() => onDelete(preset.name)}>
                {t("manual.delete")}
              </Button>
            </div>
          </li>
        );
      })}
    </ul>
  );
}
