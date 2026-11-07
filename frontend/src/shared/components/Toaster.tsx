import { useEffect, useState } from "preact/hooks";
import { dismiss, watchToasts, type Toast } from "../toast";

/** The corner where notices appear. A click removes one. */
export default function Toaster() {
  const [toasts, setToasts] = useState<Toast[]>([]);
  useEffect(() => watchToasts(setToasts), []);

  return (
    <ol aria-live="polite" className="fixed bottom-4 right-4 z-50 flex w-80 max-w-[calc(100vw-2rem)] flex-col gap-2">
      {toasts.map((item) => (
        <li key={item.id}>
          <button
            type="button"
            onClick={() => dismiss(item.id)}
            className="w-full rounded-lg border border-[var(--border-strong)] bg-[var(--surface-raised)] px-3.5 py-3 text-left text-[13px] text-[var(--text-primary)] shadow-lg"
          >
            <span className="block font-medium">{item.title}</span>
            {item.description && <span className="mt-0.5 block text-xs text-neutral-400">{item.description}</span>}
          </button>
        </li>
      ))}
    </ol>
  );
}
