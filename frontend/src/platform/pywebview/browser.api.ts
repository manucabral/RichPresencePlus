import { getPyApi } from "./pywebview";
import type { Browser } from "../../shared/types/browser";

export async function getInstalledBrowsers(): Promise<Browser[]> {
  const api = await getPyApi();
  return api.get_installed_browsers();
}

export async function getConnectedBrowser(): Promise<Browser | null> {
  const api = await getPyApi();
  return api.get_connected_browser();
}

export async function getCurrentCdpUrl(): Promise<string> {
  const api = await getPyApi();
  return api.get_current_cdp_url();
}

export async function launchBrowserByName(name: string): Promise<void> {
  const api = await getPyApi();
  return api.launch_browser(name);
}

export async function closeBrowser(url: string): Promise<boolean> {
  const api = await getPyApi();
  return api.close_browser(url);
}
