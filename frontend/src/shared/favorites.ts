import { useEffect, useState } from "preact/hooks";
import { toast } from "./toast";
import { getFavorites, setFavorite } from "../platform/api";
import { errorMessage } from "./errors";
import { t } from "./i18n";

/** Favorites read once and shared by every page that shows them. */
let known: Set<string> | null = null;
const listeners = new Set<(names: Set<string>) => void>();

function publish(names: Set<string>) {
  known = names;
  for (const listener of listeners) listener(names);
}

/** The user's favorite presences and a way to change them. */
export function useFavorites(): { favorites: Set<string>; toggle: (name: string) => void } {
  const [favorites, setFavorites] = useState<Set<string>>(known ?? new Set());

  useEffect(() => {
    listeners.add(setFavorites);
    if (known === null) {
      getFavorites().then(
        (names) => publish(new Set(names)),
        () => {}, // Without the list, nothing is marked; the page still works.
      );
    }
    return () => {
      listeners.delete(setFavorites);
    };
  }, []);

  function toggle(name: string) {
    const before = known ?? new Set<string>();
    const on = !before.has(name);
    const after = new Set(before);
    if (on) after.add(name);
    else after.delete(name);
    publish(after);
    setFavorite(name, on).catch((error: unknown) => {
      publish(before);
      toast(t("favorites.failed"), { description: errorMessage(error) });
    });
  }

  return { favorites, toggle };
}
