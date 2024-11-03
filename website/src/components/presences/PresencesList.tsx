import usePresencesList from "@/app/hooks/usePresencesList";
import PresenceItem from "./Presence";
import { Presence } from "@/interfaces";
import PresencesSkeleton from "../PresencesSkeleton";

export default function PresencesList({
  category,
  setSelectedPresence,
}: {
  category: string;
  setSelectedPresence: (presence: Presence) => void;
}) {
  const { presences, status } = usePresencesList({ category });

  if (status === "ERROR")
    return (
      <p>
        There was an error while fetching the presences. Please try again later.
      </p>
    );

  return status === "LOADING" ? (
    <PresencesSkeleton />
  ) : (
    <ul className="grid h-max w-full grid-cols-[repeat(auto-fit,minmax(275px,1fr))] gap-10 md:grid-cols-[repeat(auto-fit,minmax(300px,1fr))] lg:flex lg:flex-wrap">
      {presences?.length >= 1 ? (
        presences?.map((presence) => (
          <PresenceItem
            key={presence.clientId}
            presence={presence}
            setSelectedPresence={setSelectedPresence}
          />
        ))
      ) : (
        <p>No presences found for this category.</p>
      )}
    </ul>
  );
}
