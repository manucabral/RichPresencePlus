import { getPyApi } from "./pywebview";
import type {
  InstalledPresence,
  RemotePresence,
} from "../../shared/types/presence";
import type { ResultState } from "../../shared/types/resultstate";

export async function getInstalledPresences(): Promise<InstalledPresence[]> {
  const api = await getPyApi();
  const presences = await api.get_installed_presences();
  return presences;
}

export async function startPresence(name: string): Promise<void> {
  const api = await getPyApi();
  return api.start_presence(name);
}

export async function stopPresence(name: string): Promise<void> {
  const api = await getPyApi();
  return api.stop_presence(name);
}

export async function getRemotePresences(): Promise<RemotePresence[]> {
  const api = await getPyApi();
  const presences = await api.get_remote_presences();
  return presences;
}

export async function installRemotePresence(
  name: string,
): Promise<ResultState> {
  const api = await getPyApi();
  const response = await api.install_remote_presence(name);
  return response;
}

export async function removeInstalledPresence(
  name: string,
): Promise<ResultState> {
  const api = await getPyApi();
  const response = await api.remove_installed_presence(name);
  return response;
}

export async function isDiscordRunning(): Promise<boolean> {
  const api = await getPyApi();
  return api.is_discord_running();
}
