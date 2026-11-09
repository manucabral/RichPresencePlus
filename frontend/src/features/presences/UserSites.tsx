import { useState } from "preact/hooks";
import { setPresenceSites } from "../../platform/api";
import Button from "../../shared/components/Button";
import { Panel, Section, fieldClass } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useT } from "../../shared/i18n";
import { toast } from "../../shared/toast";

/** The sites the person adds to a presence made for a server of their own. */
export default function UserSites({ name, sites, onSaved }: { name: string; sites: string[]; onSaved: () => void }) {
  const { t } = useT();
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);

  async function save(next: string[]) {
    setBusy(true);
    try {
      await setPresenceSites(name, next);
      setTyped("");
      onSaved();
    } catch (error) {
      toast(t("presences.saveFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <Section title={t("presences.userSites")}>
      <Panel>
        <div className="px-4 py-3">
          <p className="text-xs text-neutral-500">{t("presences.userSitesHint")}</p>
          <form
            className="mt-2 flex gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              if (typed.trim() !== "") save([...sites, typed.trim()]);
            }}
          >
            <input
              value={typed}
              placeholder="media.home:8096"
              aria-label={t("presences.userSitesAdd")}
              className={`${fieldClass} flex-1`}
              onInput={(event) => setTyped((event.target as HTMLInputElement).value)}
            />
            <Button type="submit" loading={busy} disabled={typed.trim() === "" || sites.length >= 8}>
              {t("presences.userSitesAdd")}
            </Button>
          </form>
          {sites.length > 0 && (
            <ul className="mt-3 space-y-1">
              {sites.map((site) => (
                <li key={site} className="flex items-center justify-between text-sm text-neutral-200">
                  <span className="font-mono">{site}</span>
                  <Button
                    variant="ghost"
                    className="text-xs"
                    disabled={busy}
                    onClick={() => save(sites.filter((other) => other !== site))}
                  >
                    {t("presences.userSitesRemove")}
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </Panel>
    </Section>
  );
}
