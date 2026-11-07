import { useState } from "preact/hooks";
import { getDevStatus, getPresences } from "../../platform/api";
import { useLoaded } from "../../shared/hooks";
import { useSearch } from "../../shared/router";

/** The presence the Developer page is working on, shared by its tabs. */
export function usePick() {
  const presences = useLoaded(getPresences);
  const status = useLoaded(getDevStatus, 1500);
  // A presence page links here with ?presence=<name>; an unknown name is ignored below.
  const linked = useSearch().get("presence");
  const [picked, setPicked] = useState(linked ?? "");

  const all = presences.data ?? [];
  const mine = all.filter((presence) => presence.editable);
  const watching = status.data?.watching ?? null;
  // The watched presence wins over the picker, so the page shows what is true.
  const name = watching ?? (all.some((p) => p.name === picked) ? picked : (mine[0]?.name ?? all[0]?.name ?? ""));
  const canWatch = mine.some((presence) => presence.name === name);

  return {
    all,
    mine,
    name,
    /** The picked presence when it can be edited, or else the first one that can. */
    own: canWatch ? name : (mine[0]?.name ?? ""),
    canWatch,
    watching,
    status,
    linked: linked !== null,
    loaded: presences.data !== null,
    setPicked,
    reload: presences.reload,
  };
}

export type Pick = ReturnType<typeof usePick>;
