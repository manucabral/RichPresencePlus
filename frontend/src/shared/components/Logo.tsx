import { useState } from "preact/hooks";
import { initial, readableOn } from "../presence";

interface Props {
  name: string;
  /** `#RRGGBB` behind the letter; neutral grey when missing. */
  color: string | null;
  /** An `https:` or `data:` image address. */
  icon: string | null;
}

/** The logo of a presence, or its first letter on its color. */
export default function Logo({ name, color, icon }: Props) {
  const [broken, setBroken] = useState(false);
  const background = color ?? "#404040";
  const picture = icon !== null && !broken;
  return (
    <span
      className="w-9 h-9 shrink-0 rounded-lg overflow-hidden flex items-center justify-center text-sm font-semibold"
      // The color is for the letter: behind a picture it would fill its transparent parts.
      style={picture ? undefined : { background, color: readableOn(background) }}
      aria-hidden="true"
    >
      {picture ? (
        <img
          src={icon}
          alt=""
          loading="lazy"
          decoding="async"
          referrerpolicy="no-referrer"
          className="w-full h-full object-cover"
          onError={() => setBroken(true)}
        />
      ) : (
        initial(name)
      )}
    </span>
  );
}
