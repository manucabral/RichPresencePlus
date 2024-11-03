"use client";
import { useEffect, useRef, useState } from "react";
import Filters from "@/components/presences/Filters";
import MobileFilterSidebar from "./presences/MobileFilterSidebar";
import Header from "@/components/presences/Header";

export default function PresencesFilters({
  setCategory,
  category,
}: {
  setCategory: (filter: string) => void;
  category: string;
}) {
  const formRef = useRef<HTMLFormElement | null>(null);
  const [mobileFiltersOpen, setMobileFiltersOpen] = useState(false);

  const handleChange = () => {
    if (formRef.current) {
      const formData = new FormData(formRef.current);
      const filterValue = formData.get("category")!;
      setCategory(filterValue as string);
    }
  };

  return (
    <>
      <Header onClick={() => setMobileFiltersOpen(true)} />
      <MobileFilterSidebar
        open={mobileFiltersOpen}
        onClose={() => setMobileFiltersOpen(false)}
        defaultFilter={category}
      />
      <form
        className="filters hidden lg:block"
        onChange={handleChange}
        ref={formRef}
      >
        <Filters defaultFilter={category} />
      </form>
    </>
  );
}
