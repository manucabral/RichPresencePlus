// Addresses inside the window. Names are encoded, so a presence called
// "a/b?c" stays one path segment and never reaches another page.

/** The page of an installed presence. */
export function presencePath(name: string): string {
  return `/presences/${encodeURIComponent(name)}`;
}

/** The page of a catalog item. */
export function catalogPath(name: string): string {
  return `/catalog/${encodeURIComponent(name)}`;
}

/** Whether a page belongs to a sidebar section: `/presences/YouTube` is under Presences. */
export function inSection(pathname: string, path: string): boolean {
  if (path === "/") return pathname === "/";
  const under = (base: string) => pathname === base || pathname.startsWith(`${base}/`);
  // Adding presences is the second tab of Presences.
  return under(path) || (path === "/presences" && under("/catalog"));
}
