import { FilterOptionProps } from "@/interfaces";
import Filter from "./Filter";
import { FILTERS } from "@/constants/FILTERS";

const Filters = ({
  defaultFilter,
}: {
  defaultFilter: FilterOptionProps["label"];
}) => {
  return (
    <div className="p-4 py-6 lg:p-0">
      <>
        <h3 className="-my-3 flow-root">
          <span className="font-medium text-white">Categories</span>
        </h3>
        <div className="pt-6">
          <div className="space-y-4">
            {FILTERS.map((filter, filterId) => (
              <Filter
                filter={filter}
                key={filterId}
                defaultFilter={defaultFilter}
              />
            ))}
          </div>
        </div>
      </>
    </div>
  );
};

export default Filters;
