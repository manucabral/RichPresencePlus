import SortOption from "@/components/presences/SortOption";
import { ChevronDownIcon } from "@heroicons/react/20/solid";
import { Menu, Transition } from "@headlessui/react";
import { Fragment, type FC } from "react";
import { SORT_OPTIONS } from "@/constants/SORT_OPTIONS";
import { Suspense } from "react";

const Sort: FC = () => {
  return (
    <Menu as="div" className="relative inline-block text-left">
      <div>
        <Menu.Button className="group inline-flex justify-center text-sm font-medium text-white hover:text-green-1">
          Sort
          <ChevronDownIcon
            className="-mr-1 ml-1 h-5 w-5 flex-shrink-0 text-gray-400 group-hover:text-gray-500"
            aria-hidden="true"
          />
        </Menu.Button>
      </div>
      <Transition
        as={Fragment}
        enter="transition ease-out duration-100"
        enterFrom="transform opacity-0 scale-95"
        enterTo="transform opacity-100 scale-100"
        leave="transition ease-in duration-75"
        leaveFrom="transform opacity-100 scale-100"
        leaveTo="transform opacity-0 scale-95"
      >
        <Menu.Items className="absolute right-0 z-10 mt-2 w-40 origin-top-right rounded-md bg-dark-2 shadow-2xl ring-1 ring-black ring-opacity-5 focus:outline-none">
          <div className="py-1">
            {SORT_OPTIONS.map((option) => (
              <Suspense key={option.name} fallback={<>Loading...</>}>
                <SortOption option={option} />
              </Suspense>
            ))}
          </div>
        </Menu.Items>
      </Transition>
    </Menu>
  );
};

export default Sort;
