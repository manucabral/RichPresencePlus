import type { ComponentChildren } from "preact";
import { selectClass } from "../../shared/components/ui";
import { useT } from "../../shared/i18n";
import type { Pick } from "./usePick";

interface Props {
  pick: Pick;
  children: (name: string) => ComponentChildren;
}

/** A tab that works on one of the user's own presences, with its picker above. */
export default function OwnPanel({ pick, children }: Props) {
  const { t } = useT();
  if (pick.own === "") return <p className="text-[13px] text-neutral-500">{t("developer.needOwn")}</p>;
  return (
    <div className="space-y-3">
      <select
        value={pick.own}
        disabled={pick.watching !== null}
        className={selectClass}
        aria-label={t("developer.pick")}
        onChange={(event) => pick.setPicked((event.target as HTMLSelectElement).value)}
      >
        {pick.mine.map((presence) => (
          <option key={presence.name} value={presence.name}>
            {presence.name}
          </option>
        ))}
      </select>
      {children(pick.own)}
    </div>
  );
}
