import { Presence } from "@/interfaces";
import Image from "next/image";
import { useEffect, useRef } from "react";
import { XMarkIcon } from "@heroicons/react/24/solid";

export default function PresenceDetailsModal({
  presence,
  setSelectedPresence,
}: {
  presence: Presence;
  setSelectedPresence: (presence: Presence | null) => void;
}) {
  const { image, name, inProgress, author, description, version, category } =
    presence;
  const modalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        modalRef.current &&
        !modalRef.current.contains(event.target as Node)
      ) {
        setSelectedPresence(null);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [setSelectedPresence]);

  return (
    <div className="fixed inset-0 z-10 flex h-screen w-full items-center justify-center bg-[#000000ce] px-6">
      <div
        className="relative flex w-full max-w-[800px] items-center rounded-lg border border-green-10 bg-dark p-8"
        ref={modalRef}
      >
        <button
          className="group absolute right-8 top-8 rounded-full p-1 hover:bg-green-10"
          onClick={() => setSelectedPresence(null)}
        >
          <XMarkIcon className="size-7 group-hover:text-green-18" />
        </button>
        <article className="flex flex-col gap-5">
          <header className="flex flex-col gap-4 sm:flex-row">
            <div className="flex items-center justify-center rounded-full sm:items-start">
              <div className="relative h-14 w-14 rounded-full lg:h-16 lg:w-16">
                <Image
                  className="object-cover"
                  src={image}
                  fill
                  alt={`Thumbnail of ${name}`}
                  unoptimized
                />
              </div>
            </div>
            <div className="flex flex-col items-center gap-5 sm:items-start md:gap-0 xl:w-2/3">
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
              <div className="flex flex-wrap items-center gap-3">
                <span className="text-sm text-gray-400 sm:text-base">
                  {author}
                </span>
                <div className="flex h-5 items-center justify-center rounded-full bg-green-1 px-3 text-sm text-green-18">
                  {version}
                </div>
                <div className="flex h-5 items-center justify-center rounded-full bg-green-1 px-3 text-sm text-green-18">
                  {category}
                </div>
              </div>
            </div>
          </header>
          <footer>
            <p className="text-sm lg:text-base">{description?.en}</p>
          </footer>
        </article>
      </div>
    </div>
  );
}
