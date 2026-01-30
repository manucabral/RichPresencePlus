import { getPyApi } from "./pywebview";
import type { NetworkProcess } from "../../shared/types/networkprocess";

export async function getNetworkProcceses(): Promise<
  Record<number, NetworkProcess[]>
> {
  const api = await getPyApi();
  return api.get_network_processes();
}

export async function closeNetworkProcesses(port: number): Promise<boolean> {
  const api = await getPyApi();
  return api.close_network_processes(port);
}
