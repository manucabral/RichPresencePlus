import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "preact/hooks";
import { getWindowVisible } from "../platform/api";
import { errorMessage } from "./errors";
import { createScreen } from "./screen";

/** Loads that read again when something changed outside the window, such as from the tray. */
const reloaders = new Set<() => void>();
const reloadAll = () => {
  for (const reload of reloaders) reload();
};
listen("app-changed", reloadAll).catch(() => {});

/**
 * The web view still reports itself as visible after the window is hidden to
 * the tray or minimized, so the desktop side says when the window is on screen.
 * The window can start hidden, so the first value is asked for too.
 */
const screen = createScreen();
listen<boolean>("window-visible", (event) => {
  // Refreshes were skipped while away; what shows now must not wait a turn.
  if (screen.set(event.payload)) reloadAll();
}).catch(() => {
  // Outside the desktop shell there is no event; the page counts as visible.
});
getWindowVisible().then(
  (visible) => {
    screen.set(visible);
  },
  () => {},
);

export interface Loaded<T> {
  data: T | null;
  error: string | null;
  reload: () => void;
}

/**
 * Loads data now and again every `every` milliseconds while the page is open.
 * A failed refresh keeps the last data and shows the error. Refreshes are
 * skipped while the window is hidden.
 */
export function useLoaded<T>(load: () => Promise<T>, every = 0): Loaded<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const loader = useRef(load);
  loader.current = load;

  useEffect(() => {
    let alive = true;
    const run = () =>
      loader.current().then(
        (value) => {
          if (!alive) return;
          // Equal data keeps the same object, so polling does not redraw the page.
          setData((old) => (JSON.stringify(old) === JSON.stringify(value) ? old : value));
          setError(null);
        },
        (failure) => alive && setError(errorMessage(failure)),
      );
    run();
    reloaders.add(run);
    // A window hidden in the tray or minimized does not need fresh data.
    const timer = every > 0 ? setInterval(() => screen.on && !document.hidden && run(), every) : undefined;
    return () => {
      alive = false;
      reloaders.delete(run);
      if (timer) clearInterval(timer);
    };
  }, [every, tick]);

  return { data, error, reload: () => setTick((value) => value + 1) };
}
