import type { FilterOptionProps } from "@/interfaces";

export default function Filter({
  filter,
  defaultFilter,
}: {
  filter: FilterOptionProps;
  defaultFilter: FilterOptionProps["value"];
}) {
  return (
    <div className="flex items-center">
      <input
        id={filter.value}
        name={"category"}
        type="radio"
        defaultChecked={defaultFilter === filter.value}
        value={filter.value}
        className="h-4 w-4 rounded border-gray-300 text-green-1 focus:ring-transparent"
      />
      <label
        htmlFor={filter.value}
        className="ml-3 cursor-pointer text-sm text-gray-200 hover:text-green-1"
      >
        {filter.label}
      </label>
    </div>
  );
}
