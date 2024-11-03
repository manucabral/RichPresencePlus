"use client";
import PresenceDetailsModal from "@/components/presences/PresenceDetailsModal";
import PresencesList from "@/components/presences/PresencesList";
import PresencesFilters from "@/components/PresencesFilters";
import Section from "@/components/Section";
import { SectionsEnum } from "@/enums.d";
import { Presence } from "@/interfaces";
import { Suspense, useState } from "react";

const Presences = () => {
  const [category, setCategory] = useState("all");
  const [selectedPresence, setSelectedPresence] = useState<Presence | null>(
    null,
  );

  return (
    <>
      <Section className="mt-16 !h-full">
        <Section.Container
          id={SectionsEnum?.PRESENCES?.toLowerCase()}
          offset={-100}
        >
          <div className="mx-auto grid w-full max-w-layout grid-cols-[200px,1fr] grid-rows-[repeat(2,auto)] gap-x-8 sm:grid-cols-[250px,1fr]">
            <Suspense fallback={<>Loading...</>}>
              <PresencesFilters category={category} setCategory={setCategory} />
            </Suspense>
            <div className="presences">
              <Suspense fallback={<>Loading...</>}>
                <PresencesList
                  category={category}
                  setSelectedPresence={setSelectedPresence}
                />
              </Suspense>
            </div>
          </div>
        </Section.Container>
      </Section>
      {selectedPresence && (
        <PresenceDetailsModal
          presence={selectedPresence}
          setSelectedPresence={setSelectedPresence}
        />
      )}
    </>
  );
};

export default Presences;
