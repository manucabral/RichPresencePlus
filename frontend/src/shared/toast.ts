/** A short notice in the corner of the window. */
export interface Toast {
  id: number;
  title: string;
  description?: string;
}

/** How long a notice stays on screen. */
const SHOWN_FOR = 4000;
/** Notices on screen at once; a new one pushes the oldest out. */
const MOST = 3;

let shown: Toast[] = [];
let next = 1;
const watchers = new Set<(toasts: Toast[]) => void>();

function publish(toasts: Toast[]) {
  shown = toasts;
  for (const watcher of watchers) watcher(shown);
}

/** Removes a notice, by hand or when its time ends. */
export function dismiss(id: number) {
  publish(shown.filter((item) => item.id !== id));
}

/** Shows a notice; the description is usually the message of an error. */
export function toast(title: string, options: { description?: string } = {}) {
  const id = next++;
  publish([...shown, { id, title, description: options.description }].slice(-MOST));
  setTimeout(() => dismiss(id), SHOWN_FOR);
}

/** Calls `watcher` with the notices after each change; returns a function that stops it. */
export function watchToasts(watcher: (toasts: Toast[]) => void): () => void {
  watchers.add(watcher);
  return () => watchers.delete(watcher);
}
