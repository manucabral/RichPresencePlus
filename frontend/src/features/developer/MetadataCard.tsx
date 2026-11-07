import { useEffect, useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { deletePresence, updateManifest } from "../../platform/api";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useT } from "../../shared/i18n";
import type { ManifestChanges, PresenceView } from "../../shared/types";
import TagPicker from "./TagPicker";

const field = "rounded-md border border-neutral-800 bg-neutral-950 px-3 py-1.5 text-[13px] text-neutral-100 placeholder:text-neutral-600";
const HEX = /^#[0-9a-fA-F]{6}$/;

interface Form {
  version: string;
  description: string;
  author: string;
  color: string;
  tags: string[];
}

function formOf(presence: PresenceView): Form {
  return {
    version: presence.version,
    description: presence.description ?? "",
    author: presence.author ?? "",
    color: presence.color ?? "",
    tags: presence.tags,
  };
}

/** Only the fields the person touched, so a translated description is not written back by accident. */
function changesOf(before: Form, after: Form): ManifestChanges {
  const changes: ManifestChanges = {};
  if (after.version !== before.version) changes.version = after.version;
  if (after.description !== before.description) changes.description = after.description;
  if (after.author !== before.author) changes.author = after.author;
  if (after.color !== before.color) changes.color = after.color;
  if (after.tags.join() !== before.tags.join()) changes.tags = after.tags;
  return changes;
}

interface Props {
  presence: PresenceView;
  /** Called after it was saved or deleted, so the page reads the list again. */
  onChanged: () => void;
}

/** Edits author, color, tags and version of a presence in the user's own folder, or deletes it. */
export default function MetadataCard({ presence, onChanged }: Props) {
  const { t } = useT();
  const current = formOf(presence);
  const [form, setForm] = useState(current);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);

  // The manifest can change in the editor while this is open. Keeping the old
  // values here would offer to write them over the new ones.
  const written = JSON.stringify(current);
  useEffect(() => setForm(current), [written]);

  const changes = changesOf(current, form);
  const colorOk = form.color === "" || HEX.test(form.color);
  const set = (key: Exclude<keyof Form, "tags">) => (event: Event) =>
    setForm({ ...form, [key]: (event.target as HTMLInputElement).value });

  async function save() {
    setBusy(true);
    try {
      await updateManifest(presence.name, changes);
      onChanged();
      toast(t("developer.aboutSaved"));
    } catch (error) {
      toast(t("developer.aboutFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    const { name: deleted } = presence;
    setBusy(true);
    try {
      await deletePresence(deleted);
      toast(t("developer.deleted", { name: deleted }));
      setConfirming(false);
      onChanged();
    } catch (error) {
      toast(t("developer.deleteFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="p-4 rounded-lg bg-neutral-900/40 border border-neutral-800 space-y-3">
      <div>
        <p className="text-sm text-neutral-200">{t("developer.about")}</p>
        <p className="text-xs text-neutral-500">{t("developer.aboutHint")}</p>
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        <label className="space-y-1 text-xs text-neutral-500">
          {t("developer.version")}
          <input className={`w-full ${field}`} value={form.version} maxLength={32} onInput={set("version")} />
        </label>
        <label className="space-y-1 text-xs text-neutral-500">
          {t("developer.author")}
          <input className={`w-full ${field}`} value={form.author} maxLength={64} onInput={set("author")} />
        </label>
        <label className="space-y-1 text-xs text-neutral-500 sm:col-span-2">
          {t("developer.description")}
          <input
            className={`w-full ${field}`}
            value={form.description}
            maxLength={256}
            onInput={set("description")}
          />
        </label>
        <label className="space-y-1 text-xs text-neutral-500">
          {t("developer.color")}
          <span className="flex gap-2">
            <input
              type="color"
              className="h-8 w-10 rounded bg-transparent"
              value={HEX.test(form.color) ? form.color : "#404040"}
              aria-label={t("developer.color")}
              onInput={set("color")}
            />
            <input
              className={`flex-1 min-w-0 ${field} ${colorOk ? "" : "border-danger"}`}
              value={form.color}
              maxLength={7}
              placeholder="#RRGGBB"
              onInput={set("color")}
            />
          </span>
        </label>
        <div className="sm:col-span-2">
          <TagPicker chosen={form.tags} onChange={(tags) => setForm({ ...form, tags })} />
        </div>
      </div>
      <Button
        variant="primary"
        loading={busy}
        disabled={!colorOk || Object.keys(changes).length === 0}
        onClick={save}
      >
        {t("developer.aboutSave")}
      </Button>
      <div className="space-y-2 border-t border-neutral-800 pt-3">
        <div>
          <p className="text-sm text-neutral-200">{t("developer.delete")}</p>
          <p className="text-xs text-neutral-500">{t("developer.deleteHint")}</p>
        </div>
        {confirming ? (
          <div className="flex flex-wrap gap-2">
            <Button variant="danger" loading={busy} onClick={remove}>
              {t("developer.deleteSure", { name: presence.name })}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={() => setConfirming(false)}>
              {t("developer.deleteKeep")}
            </Button>
          </div>
        ) : (
          <Button variant="danger" onClick={() => setConfirming(true)}>
            {t("developer.delete")}
          </Button>
        )}
      </div>
    </section>
  );
}
