/**
 * The installer of the latest release, asked for when the site is built. The
 * request runs on the build machine; no token ever reaches a visitor.
 */

const LATEST = "https://api.github.com/repos/manucabral/RichPresencePlus/releases/latest";

/** Page that lists every release; the way out when there is no installer. */
export const RELEASES = "https://github.com/manucabral/RichPresencePlus/releases";

export interface Installer {
  version: string;
  url: string;
  /** Size in megabytes, rounded. */
  size: number;
}

interface Asset {
  name: string;
  browser_download_url: string;
  size: number;
}

interface Release {
  tag_name?: string;
  assets?: Asset[];
}

/** The installer of a release. */
export function installerOf(release: Release): Installer | null {
  const asset = release.assets?.find((file) => file.name.endsWith("-setup.exe"));
  if (!asset || !release.tag_name) return null;
  return {
    version: release.tag_name.replace(/^v/, ""),
    url: asset.browser_download_url,
    size: Math.max(1, Math.round(asset.size / (1024 * 1024))),
  };
}

/** Asks GitHub for the latest installer. */
export async function latestInstaller(ask: typeof fetch = fetch): Promise<Installer | null> {
  const token = process.env.GITHUB_TOKEN;
  try {
    const response = await ask(LATEST, {
      headers: {
        Accept: "application/vnd.github+json",
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      },
      signal: AbortSignal.timeout(5000),
    });
    return response.ok ? installerOf((await response.json()) as Release) : null;
  } catch {
    return null;
  }
}
