"use client";
import type { SortOptionProps } from "@/interfaces";
import { Menu } from "@headlessui/react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import type { FC } from "react";

const SortOption: FC<SortOptionProps> = ({ option }) => {
  const { push } = useRouter();
  const searchParams = useSearchParams();
  const pathname = usePathname();

  const handleClick = (value: string) => {
    const params = new URLSearchParams(searchParams as any);
    params.set("sortBy", value.toString());
    push(`${pathname}?${params.toString()}`);
  };

  return (
    <Menu.Item key={option.name}>
      {({ active }) => (
        <p
          className={`block cursor-pointer border border-transparent px-4 py-2 text-sm hover:text-green-1 ${searchParams.get("sortBy") === option?.value && "!border-green-1 text-green-1"}`}
          onClick={() => handleClick(option?.value)}
        >
          {option.name}
        </p>
      )}
    </Menu.Item>
  );
};

export default SortOption;
