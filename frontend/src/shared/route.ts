/** Where the interface is, read from the address after the `#`. */
export interface Place {
  pathname: string;
  search: string;
}

/**
 * Reads a place from the hash of the address, with or without its `#`. The
 * path always starts with `/`, and the search keeps its `?`.
 */
export function placeOf(hash: string): Place {
  const text = hash.replace(/^#/, "");
  const mark = text.indexOf("?");
  const path = mark < 0 ? text : text.slice(0, mark);
  return {
    pathname: path.startsWith("/") ? path : `/${path}`,
    search: mark < 0 ? "" : text.slice(mark),
  };
}

function decoded(part: string): string {
  try {
    return decodeURIComponent(part);
  } catch {
    // A hand-written address can hold a broken escape; it stays as typed.
    return part;
  }
}

/**
 * Matches a path against a pattern such as `/presences/:name`, where a part
 * starting with `:` takes any one part. Null when it does not match.
 */
export function matchPath(pattern: string, pathname: string): Record<string, string> | null {
  const expected = pattern.split("/").filter(Boolean);
  const actual = pathname.split("/").filter(Boolean);
  if (expected.length !== actual.length) return null;
  const params: Record<string, string> = {};
  for (const [index, part] of expected.entries()) {
    if (part.startsWith(":")) params[part.slice(1)] = decoded(actual[index]);
    else if (part !== actual[index]) return null;
  }
  return params;
}
