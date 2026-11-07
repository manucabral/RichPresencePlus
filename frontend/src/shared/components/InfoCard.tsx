import Linked from "./Linked";

interface InfoCardProps {
  text: string;
}

export default function InfoCard({ text }: InfoCardProps) {
  return (
    <div className="mt-2 rounded-lg border border-neutral-800 bg-neutral-900/40 px-3 py-2.5">
      <div className="flex items-start gap-3">
        <svg
          className="mt-0.5 h-4 w-4 shrink-0 text-neutral-500"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={1.5}
            d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        <p className="text-xs leading-relaxed text-neutral-500">
          <Linked text={text} />
        </p>
      </div>
    </div>
  );
}
