import { hasCodeEditor, openInCodeEditor, openPresenceFolder } from "../../platform/api";
import Accordion from "../../shared/components/Accordion";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { toast } from "../../shared/toast";
import type { PresenceView } from "../../shared/types";
import MetadataCard from "./MetadataCard";
import ReloadCard from "./ReloadCard";
import VersionsPanel from "./VersionsPanel";

const FILES = ["manifest.json", "main.js", "locales"];

interface Props {
  presence: PresenceView;
  /** Called after its details change or it is deleted. */
  onChanged: () => void;
}

/** Everything about writing one presence: its files, its details and its saved versions. */
export default function CodePanel({ presence, onChanged }: Props) {
  const { t } = useT();
  const editor = useLoaded(hasCodeEditor);
  const { name } = presence;

  async function open(action: (name: string) => Promise<void>) {
    try {
      await action(name);
    } catch (error) {
      toast(t("presences.folderFailed"), { description: errorMessage(error) });
    }
  }

  return (
    <>
      <section className="space-y-3 rounded-lg border border-neutral-800 bg-neutral-900/40 p-4">
        <div>
          <p className="text-sm text-neutral-200">{t("developer.codeTitle")}</p>
          <p className="text-xs text-neutral-500">{t("developer.codeHint")}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          {editor.data && (
            <Button variant="primary" onClick={() => open(openInCodeEditor)}>
              {t("developer.openEditor")}
            </Button>
          )}
          <Button variant={editor.data ? "neutral" : "primary"} onClick={() => open(openPresenceFolder)}>
            {t("developer.openFolder")}
          </Button>
        </div>
        {editor.data === false && <p className="text-xs text-neutral-500">{t("developer.noEditor")}</p>}
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
          {FILES.map((file) => (
            <div key={file} className="contents">
              <dt className="font-mono text-neutral-300">{file}</dt>
              <dd className="text-neutral-500">{t(`developer.file.${file}`)}</dd>
            </div>
          ))}
        </dl>
      </section>
      <MetadataCard key={name} presence={presence} onChanged={onChanged} />
      <Accordion title={t("developer.versions")}>
        <VersionsPanel key={name} name={name} />
      </Accordion>
      <Accordion title={t("developer.reload")}>
        <ReloadCard />
      </Accordion>
    </>
  );
}
