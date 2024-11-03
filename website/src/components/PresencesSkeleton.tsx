export default function PresencesSkeleton() {
  const items = Array.from({ length: 6 }, (_, i) => i);

  return (
    <ul className="grid h-max w-full grid-cols-[repeat(auto-fit,minmax(275px,1fr))] gap-10 md:grid-cols-[repeat(auto-fit,minmax(300px,1fr))] lg:flex lg:flex-wrap">
      {items.map((item) => (
        <li
          key={item}
          className="sm:min-h-50 flex w-full cursor-pointer flex-col gap-5 rounded-md border border-gray-700 p-5 duration-150 hover:border-green-1 lg:min-h-64 lg:max-w-[279px]"
        >
          <div className="flex flex-col gap-4 sm:flex-row">
            <div className="flex items-center justify-center rounded-full sm:items-start">
              <div className="relative h-14 w-14 animate-pulse rounded-full bg-gray-800 lg:h-16 lg:w-16"></div>
            </div>
            <div className="flex w-full flex-col gap-3">
              <div className="flex h-5 w-full animate-pulse rounded-lg bg-gray-800"></div>
              <div className="flex h-5 w-full animate-pulse rounded-lg bg-gray-800"></div>
            </div>
          </div>
          <div className="flex flex-col gap-4">
            <div className="flex h-5 w-full animate-pulse rounded-lg bg-gray-800"></div>
            <div className="flex h-5 w-full animate-pulse rounded-lg bg-gray-800"></div>
            <div className="flex h-5 w-full animate-pulse rounded-lg bg-gray-800"></div>
            <div className="flex h-5 w-full animate-pulse rounded-lg bg-gray-800"></div>
          </div>
        </li>
      ))}
    </ul>
  );
}
