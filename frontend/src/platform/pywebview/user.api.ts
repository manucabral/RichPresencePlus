import { getPyApi } from "./pywebview";

export async function getUserSettingKey(key: string): Promise<any> {
  const api = await getPyApi();
  return api.get_user_setting_key(key);
}

export async function setUserSettingKey(
  key: string,
  value: any,
): Promise<void> {
  const api = await getPyApi();
  return api.set_user_setting_key(key, value);
}
