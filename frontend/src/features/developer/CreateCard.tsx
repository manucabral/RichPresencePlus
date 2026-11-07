import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { copyPresence, createPresence, getPresences } from "../../platform/api";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import type { Template } from "../../shared/types";

const field = "rounded-md border border-neutral-800 bg-neutral-950 px-3 py-1.5 text-[13px] text-neutral-100 placeholder:text-neutral-600";

interface Props {
  /** Called with the name of a presence that was just created. */
  onCreated: (name: string) => void;
}

/** Starts a presence from a template, or from a copy of one that works. */
export default function CreateCard({ onCreated }: Props) {
  const { t } = useT();
  const presences = useLoaded(getPresences);
  const [name, setName] = useState("");
  const [template, setTemplate] = useState<Template>("web");
  const [source, setSource] = useState("");
  const [copyName, setCopyName] = useState("");
  const [busy, setBusy] = useState(false);

  async function make(label: string, action: () => Promise<string>, then: () => void) {
    setBusy(true);
    try {
      const folder = await action();
      toast(t("developer.created", { folder }));
      then();
      onCreated(label);
    } catch (error) {
      toast(t("developer.createFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  const all = presences.data ?? [];
  const chosen = all.some((presence) => presence.name === source) ? source : (all[0]?.name ?? "");

  return (
    <section className="p-4 rounded-lg bg-neutral-900/40 border border-neutral-800 space-y-4">
      <div className="space-y-2">
        <p className="text-sm text-neutral-200">{t("developer.create")}</p>
        <div className="flex flex-wrap gap-2">
          <input
            className={`flex-1 min-w-40 ${field}`}
            value={name}
            maxLength={64}
            placeholder={t("developer.name")}
            aria-label={t("developer.name")}
            onInput={(event) => setName((event.target as HTMLInputElement).value)}
          />
          <select
            className={field}
            value={template}
            aria-label={t("developer.template")}
            onChange={(event) => setTemplate((event.target as HTMLSelectElement).value as Template)}
          >
            <option value="web">{t("developer.template.web")}</option>
            <option value="desktop">{t("developer.template.desktop")}</option>
          </select>
          <Button
            variant="primary"
            loading={busy}
            disabled={name.trim() === ""}
            onClick={() => make(name.trim(), () => createPresence(name.trim(), template), () => setName(""))}
          >
            {t("developer.createButton")}
          </Button>
        </div>
      </div>

      <div className="space-y-2">
        <p className="text-sm text-neutral-200">{t("developer.copyFrom")}</p>
        <p className="text-xs text-neutral-500">{t("developer.copyHint")}</p>
        <div className="flex flex-wrap gap-2">
          <select
            className={field}
            value={chosen}
            aria-label={t("developer.pick")}
            onChange={(event) => setSource((event.target as HTMLSelectElement).value)}
          >
            {all.map((presence) => (
              <option key={presence.name} value={presence.name}>
                {presence.name}
              </option>
            ))}
          </select>
          <input
            className={`flex-1 min-w-40 ${field}`}
            value={copyName}
            maxLength={64}
            placeholder={t("developer.newName")}
            aria-label={t("developer.newName")}
            onInput={(event) => setCopyName((event.target as HTMLInputElement).value)}
          />
          <Button
            variant="neutral"
            loading={busy}
            disabled={copyName.trim() === "" || chosen === ""}
            onClick={() =>
              make(copyName.trim(), () => copyPresence(chosen, copyName.trim()), () => setCopyName(""))
            }
          >
            {t("developer.copyButton")}
          </Button>
        </div>
      </div>

    </section>
  );
}
