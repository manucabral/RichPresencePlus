import Sort from "./Sort";
import { FunnelIcon } from "@heroicons/react/20/solid";
import { type FC } from "react";
import type { PresencesHeaderProps } from "@/interfaces";

const Header: FC<PresencesHeaderProps> = ({ onClick }) => {
  return (
    <header className="header flex w-full flex-col items-baseline justify-between gap-5 pb-16 sm:flex-row sm:gap-0">
      <h1 className="w-full text-center text-2xl font-bold tracking-tight text-white md:text-3xl">
        Presences list
      </h1>
      <div className="flex items-center self-end sm:self-start">
        {/* <Sort /> */}
        <button
          type="button"
          className="-m-2 ml-4 p-2 text-white hover:text-green-1 sm:ml-6 lg:hidden"
          onClick={onClick}
        >
          <span className="sr-only">Filters</span>
          <FunnelIcon className="h-5 w-5" aria-hidden="true" />
        </button>
      </div>
    </header>
  );
};

export default Header;
