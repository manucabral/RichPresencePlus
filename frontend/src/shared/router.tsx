import type { AnchorHTMLAttributes, ComponentChildren, ComponentType } from "preact";
import { createContext } from "preact";
import { useContext, useEffect, useState } from "preact/hooks";
import { matchPath, placeOf, type Place } from "./route";

const Params = createContext<Record<string, string>>({});

/** The current place; a component using it follows every navigation. */
export function useLocation(): Place {
  const [place, setPlace] = useState(() => placeOf(window.location.hash));
  useEffect(() => {
    const follow = () => {
      const next = placeOf(window.location.hash);
      setPlace((now) => (now.pathname === next.pathname && now.search === next.search ? now : next));
    };
    window.addEventListener("hashchange", follow);
    // The address can change between the first drawing and this line.
    follow();
    return () => window.removeEventListener("hashchange", follow);
  }, []);
  return place;
}

/** The named parts of the current path. */
export function useParams(): Record<string, string> {
  return useContext(Params);
}

/** The options of the current address. */
export function useSearch(): URLSearchParams {
  return new URLSearchParams(useLocation().search);
}

type LinkProps = Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href" | "role"> & { to: string };

/** A link inside the application: the browser moves the hash by itself. */
export function Link({ to, ...rest }: LinkProps) {
  return <a href={`#${to}`} {...rest} />;
}

export type Route = [pattern: string, Page: ComponentType];

/** Shows the first route that matches, or the fallback. */
export function Pages({ routes, fallback }: { routes: Route[]; fallback: ComponentChildren }) {
  const { pathname } = useLocation();
  for (const [pattern, Page] of routes) {
    const params = matchPath(pattern, pathname);
    if (params) {
      return (
        <Params.Provider value={params}>
          <Page />
        </Params.Provider>
      );
    }
  }
  return <>{fallback}</>;
}
