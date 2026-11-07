import { useEffect, useState } from "preact/hooks";
import { toast } from "../../shared/toast";
import { startRotation, stopRotation } from "../../platform/api";
import Accordion from "../../shared/components/Accordion";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { useT } from "../../shared/i18n";
import type { ManualPreset, Rotation } from "../../shared/types";

/** The app accepts 15 seconds to an hour per status. */
const MIN_SECONDS = 15;
const MAX_SECONDS = 3600;

interface Props {
  presets: ManualPreset[];
  rotation: Rotation | null;
  onChange: () => void;
}

/** Choose saved statuses and how long each stays, then start or stop the rotation. */
export default function RotationCard({ presets, rotation, onChange }: Props) {
  const { t } = useT();
  const [chosen, setChosen] = useState<string[]>(rotation?.names ?? []);
  const [seconds, setSeconds] = useState(rotation?.seconds ?? 30);
  const [busy, setBusy] = useState(false);

  // A rotation started or ended elsewhere (the tray, a restart) shows here.
  useEffect(() => {
    if (rotation) {
      setChosen(rotation.names);
      setSeconds(rotation.seconds);
    }
  }, [rotation]);

  // A status deleted since it was ticked is dropped from the choice.
  const names = chosen.filter((name) => presets.some((preset) => preset.name === name));
  const valid = names.length >= 2 && seconds >= MIN_SECONDS && seconds <= MAX_SECONDS;

  function toggle(name: string) {
    setChosen(names.includes(name) ? names.filter((n) => n !== name) : [...names, name]);
  }

  async function run(action: () => Promise<void>) {
    setBusy(true);
    try {
      await action();
      onChange();
    } catch (error) {
      toast(t("manual.actionFailed"), { description: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }

  if (presets.length < 2) return null;

  return (
    <Accordion open={rotation !== null} title={rotation ? t("manual.rotating", { seconds: rotation.seconds }) : t("manual.rotation")}>
      <div className="space-y-3">
        <p className="text-xs text-neutral-500">{t("manual.rotationHint")}</p>
        <ul className="flex flex-wrap gap-x-5 gap-y-1">
          {presets.map((preset) => (
            <li key={preset.name}>
              <label className="flex items-center gap-2 text-[13px] text-neutral-300">
                <input type="checkbox" checked={names.includes(preset.name)} onChange={() => toggle(preset.name)} />
                {names.includes(preset.name) && (
                  <span className="text-xs tabular-nums text-neutral-500">{names.indexOf(preset.name) + 1}.</span>
                )}
                {preset.name}
              </label>
            </li>
          ))}
        </ul>
        <div className="flex flex-wrap items-center gap-2">
          <label className="flex items-center gap-2 text-[13px] text-neutral-300">
            {t("manual.rotationSeconds")}
            <input
              type="number"
              min={MIN_SECONDS}
              max={MAX_SECONDS}
              value={seconds}
              className="w-20 rounded-md border border-neutral-800 bg-neutral-950 px-2 text-neutral-100"
              onInput={(event) => setSeconds(Number((event.target as HTMLInputElement).value))}
            />
          </label>
          <Button variant="primary" className="text-xs" disabled={!valid} loading={busy} onClick={() => run(() => startRotation(names, seconds))}>
            {rotation ? t("manual.rotationRestart") : t("manual.rotationStart")}
          </Button>
          {rotation && (
            <Button variant="ghost" className="text-xs" disabled={busy} onClick={() => run(stopRotation)}>
              {t("manual.rotationStop")}
            </Button>
          )}
        </div>
      </div>
    </Accordion>
  );
}
