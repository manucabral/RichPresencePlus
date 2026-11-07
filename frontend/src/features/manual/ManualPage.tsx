import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { deleteManualPreset, exportManualPreset, getManual, saveManualPreset, showManual } from "../../platform/api";
import ActivityCard from "../../shared/components/ActivityCard";
import PageTitle from "../../shared/components/PageTitle";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import type { ManualPreset } from "../../shared/types";
import ImportCard from "./ImportCard";
import PresetForm from "./PresetForm";
import PresetList from "./PresetList";
import RotationCard from "./RotationCard";
import { blankPreset } from "./preset";
import { pageWidth } from "../../shared/components/ui";

export default function ManualPage() {
  const { t } = useT();
  // A rotation changes the status on display, so the page reads it again.
  const manual = useLoaded(getManual, 5000);
  const [draft, setDraft] = useState<ManualPreset>(blankPreset);
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(false);

  async function run(action: () => Promise<void>, failure: string) {
    setBusy(true);
    try {
      await action();
      manual.reload();
      return true;
    } catch (error) {
      toast(failure, { description: errorMessage(error) });
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function save(show: boolean) {
    const done = await run(async () => {
      await saveManualPreset(draft);
      if (show) await showManual(draft.name.trim());
    }, t("manual.saveFailed"));
    if (done) setEditing(true);
  }

  function startNew() {
    setDraft(blankPreset());
    setEditing(false);
  }

  function edit(preset: ManualPreset) {
    setDraft(preset);
    setEditing(true);
  }

  async function copy(name: string) {
    try {
      await navigator.clipboard.writeText(await exportManualPreset(name));
      toast(t("manual.copied"));
    } catch (error) {
      toast(t("manual.actionFailed"), { description: errorMessage(error) });
    }
  }

  function imported(preset: ManualPreset) {
    // A new draft: saving it creates a status, or replaces one with its name.
    setDraft(preset);
    setEditing(false);
  }

  async function remove(name: string) {
    const done = await run(() => deleteManualPreset(name), t("manual.actionFailed"));
    if (done && draft.name === name) startNew();
  }

  if (!manual.data) {
    return <p className="text-sm text-neutral-500">{manual.error ?? t("app.loading")}</p>;
  }

  return (
    <div className={`${pageWidth} space-y-6`}>
      <PageTitle title={t("manual.title")} subtitle={t("manual.subtitle")} />
      <section className="space-y-2">
        <h2 className="text-[13px] font-medium text-neutral-400">{t("manual.saved")}</h2>
        <PresetList
          presets={manual.data.presets}
          active={manual.data.active}
          busy={busy}
          onShow={(name) => run(() => showManual(name), t("manual.actionFailed"))}
          onEdit={edit}
          onDelete={remove}
          onCopy={copy}
        />
        <RotationCard presets={manual.data.presets} rotation={manual.data.rotation} onChange={manual.reload} />
        <ImportCard onLoaded={imported} />
      </section>
      <div className="grid gap-6 md:grid-cols-[3fr_2fr]">
        <PresetForm draft={draft} editing={editing} busy={busy} onChange={setDraft} onSave={save} onNew={startNew} />
        <div className="space-y-2">
          <h2 className="text-[13px] font-medium text-neutral-400">{t("manual.preview")}</h2>
          <ActivityCard
            title={t("manual.previewTitle")}
            activity={draft.activity.details || draft.activity.state ? draft.activity : null}
          />
        </div>
      </div>
    </div>
  );
}
