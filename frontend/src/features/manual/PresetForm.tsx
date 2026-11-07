import Accordion from "../../shared/components/Accordion";
import Linked from "../../shared/components/Linked";
import Button from "../../shared/components/Button";
import { useT } from "../../shared/i18n";
import type { Activity, ActivityKind, ManualPreset } from "../../shared/types";
import { MAX_BUTTONS, buttonRows, filledButtons, orNull } from "./preset";

const KINDS: ActivityKind[] = ["playing", "listening", "watching", "competing"];
const field = "w-full rounded-md border border-neutral-800 bg-neutral-950 px-3 py-1.5 text-[13px] text-neutral-100 placeholder:text-neutral-600";

interface Props {
  draft: ManualPreset;
  /** True while the draft is a saved preset being edited. */
  editing: boolean;
  busy: boolean;
  onChange: (draft: ManualPreset) => void;
  onSave: (show: boolean) => void;
  onNew: () => void;
}

function Field({ label, hint, children }: { label: string; hint?: string; children: preact.ComponentChildren }) {
  return (
    <label className="block">
      <span className="text-xs text-neutral-400">{label}</span>
      {children}
      {hint && (
        <span className="block mt-1 text-[11px] text-neutral-600">
          <Linked text={hint} />
        </span>
      )}
    </label>
  );
}

export default function PresetForm({ draft, editing, busy, onChange, onSave, onNew }: Props) {
  const { t } = useT();
  const { activity } = draft;
  const setActivity = (change: Partial<Activity>) => onChange({ ...draft, activity: { ...activity, ...change } });
  const text = (key: "details" | "state" | "large_image" | "large_text" | "small_image" | "small_text") => (
    <input
      className={field}
      value={activity[key] ?? ""}
      onInput={(event) => setActivity({ [key]: orNull((event.target as HTMLInputElement).value) })}
    />
  );
  const rows = buttonRows(activity.buttons);
  // A status that already uses them opens with them in view.
  const extras = Boolean(
    activity.large_image || activity.large_text || activity.small_image || activity.small_text || activity.buttons.length > 0,
  );
  const setButton = (index: number, change: Partial<(typeof rows)[number]>) => {
    const next = rows.map((row, at) => (at === index ? { ...row, ...change } : row));
    setActivity({ buttons: filledButtons(next) });
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-medium text-neutral-200">
          {editing ? t("manual.editing", { name: draft.name }) : t("manual.new")}
        </h2>
        {editing && (
          <Button variant="ghost" className="text-xs" onClick={onNew}>
            {t("manual.startNew")}
          </Button>
        )}
      </div>
      <Field label={t("manual.name")}>
        <input
          className={field}
          value={draft.name}
          maxLength={64}
          // A saved preset keeps its name: the name is what identifies it.
          disabled={editing}
          onInput={(event) => onChange({ ...draft, name: (event.target as HTMLInputElement).value })}
        />
      </Field>
      <Field label={t("manual.appId")} hint={t("manual.appIdHint")}>
        <input
          className={field}
          value={draft.client_id}
          inputMode="numeric"
          placeholder="123456789012345678"
          onInput={(event) => onChange({ ...draft, client_id: (event.target as HTMLInputElement).value.trim() })}
        />
      </Field>
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label={t("manual.details")}>{text("details")}</Field>
        <Field label={t("manual.state")}>{text("state")}</Field>
      </div>
      <Field label={t("manual.kind")}>
        <select
          className={field}
          value={activity.kind}
          onChange={(event) => setActivity({ kind: (event.target as HTMLSelectElement).value as ActivityKind })}
        >
          {KINDS.map((kind) => (
            <option key={kind} value={kind}>
              {t(`manual.kind.${kind}`)}
            </option>
          ))}
        </select>
      </Field>
      <Accordion open={extras} title={t("manual.extras")}>
      <div className="space-y-3">
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label={t("manual.largeImage")}>{text("large_image")}</Field>
        <Field label={t("manual.largeText")}>{text("large_text")}</Field>
        <Field label={t("manual.smallImage")}>{text("small_image")}</Field>
        <Field label={t("manual.smallText")}>{text("small_text")}</Field>
      </div>
      <fieldset className="space-y-2">
        <legend className="text-xs text-neutral-400">{t("manual.buttons", { count: MAX_BUTTONS })}</legend>
        {rows.map((row, index) => (
          <div key={index} className="grid gap-2 sm:grid-cols-[1fr_2fr]">
            <input
              className={field}
              value={row.label}
              maxLength={32}
              placeholder={t("manual.buttonLabel")}
              aria-label={t("manual.buttonLabel")}
              onInput={(event) => setButton(index, { label: (event.target as HTMLInputElement).value })}
            />
            <input
              className={field}
              value={row.url}
              placeholder="https://"
              aria-label={t("manual.buttonUrl")}
              onInput={(event) => setButton(index, { url: (event.target as HTMLInputElement).value.trim() })}
            />
          </div>
        ))}
      </fieldset>
      </div>
      </Accordion>
      <label className="flex items-center gap-2 text-sm text-neutral-300">
        <input
          type="checkbox"
          checked={draft.show_elapsed}
          onChange={(event) => onChange({ ...draft, show_elapsed: (event.target as HTMLInputElement).checked })}
        />
        {t("manual.elapsed")}
      </label>
      <div className="flex gap-2">
        <Button variant="neutral" loading={busy} onClick={() => onSave(false)}>
          {t("manual.save")}
        </Button>
        <Button variant="primary" loading={busy} onClick={() => onSave(true)}>
          {t("manual.saveAndShow")}
        </Button>
      </div>
    </div>
  );
}
