interface InfoCardProps {
  text: string;
}

export default function InfoCard({ text }: InfoCardProps) {
  return (
    <div className="mt-2 p-2 rounded-lg bg-zinc-900/40 border border-zinc-800/50">
      <div className="flex items-start gap-3">
        <svg
          className="w-4 h-4 text-zinc-500 mt-0.5"
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
        <p className="text-xs text-zinc-500 leading-relaxed">{text}</p>
      </div>
    </div>
  );
}
