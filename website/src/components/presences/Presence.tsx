import { Presence } from "@/interfaces";
import Image from "next/image";

export default function PresenceItem({
  presence,
  setSelectedPresence,
}: {
  presence: Presence;
  setSelectedPresence: (presence: Presence) => void;
}) {
  const { clientId, image, name, inProgress, author, description } = presence;

  return (
    <li
      key={clientId}
      className="sm:min-h-50 w-full cursor-pointer rounded-md border border-gray-700 p-5 duration-150 hover:border-green-1 lg:min-h-64 lg:max-w-[279px]"
      onClick={() => setSelectedPresence(presence)}
    >
      <article className="flex flex-row gap-5 sm:flex-col">
        <header className="flex flex-col items-center gap-4 sm:flex-row">
          <div className="flex items-center rounded-full sm:items-start xl:w-1/3">
            <div className="relative h-12 w-12 rounded-full sm:h-14 sm:w-14 lg:h-16 lg:w-16">
              <Image
                className="object-cover"
                src={image}
                fill
                alt={`Thumbnail of ${name}`}
                unoptimized
              />
            </div>
          </div>
          <div className="flex flex-col items-center gap-1 sm:items-start md:gap-0 xl:w-2/3">
            {inProgress && (
              <div className="flex items-center justify-center rounded-full border-[2px] border-yellow-500 px-2 py-[.7px] pb-[2px]">
                <span className="min-w-16 text-center text-xs text-yellow-400">
                  In progress
                </span>
              </div>
            )}
            <h3 className="text-base leading-none text-green-1 sm:text-lg lg:text-xl">
              {name}
            </h3>
            <span className="text-sm text-gray-400 sm:text-base">{author}</span>
          </div>
        </header>
        <footer>
          <p className="line-clamp-5 text-sm lg:text-base">{description?.en}</p>
        </footer>
      </article>
    </li>
  );
}
