import { useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { createProgramPresence, getRunningPrograms } from "../../platform/api";
import Accordion from "../../shared/components/Accordion";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { fieldClass, selectClass } from "../../shared/components/ui";
import { displayName } from "./program";

interface Props {
  /** Called after the presence exists, so the list reads it. */
  onCreated: () => void;
}

/** Makes a presence for a running program, with no code. */
export default function ProgramCard({ onCreated }: Props) {
  const { t } = useT();
  const programs = useLoaded(getRunningPrograms);
  const [picked, setPicked] = useState("");
  const [details, setDetails] = useState("");
  const [state, setState] = useState("");
  const [busy, setBusy] = useState(false);

  const list = programs.data ?? [];
  const program = list.includes(picked) ? picked : (list[0] ?? "");

  async function create() {
    setBusy(true);
    try {
      await createProgramPresence({ name: displayName(program), program, details, state });
      toast(t("program.created", { name: displayName(program) }));
      setDetails("");
      setState("");
      onCreated();
    } catch (error) {
      toast(t("program.failed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <Accordion className="mb-4" title={t("program.title")}>
      <div className="space-y-2">
        <p className="text-xs text-neutral-500">{t("program.hint")}</p>
        <select
          className={`${selectClass} w-full`}
          value={program}
          aria-label={t("program.pick")}
          onChange={(event) => setPicked((event.target as HTMLSelectElement).value)}
        >
          {list.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
        <input
          className={fieldClass}
          value={details}
          maxLength={128}
          placeholder={t("program.details")}
          aria-label={t("program.details")}
          onInput={(event) => setDetails((event.target as HTMLInputElement).value)}
        />
        <input
          className={fieldClass}
          value={state}
          maxLength={128}
          placeholder={t("program.state")}
          aria-label={t("program.state")}
          onInput={(event) => setState((event.target as HTMLInputElement).value)}
        />
        <Button variant="primary" className="text-xs" loading={busy} disabled={program === "" || details.trim().length < 2} onClick={create}>
          {t("program.create")}
        </Button>
      </div>
    </Accordion>
  );
}
