import { getLocalApi, setLocalApi } from "../../platform/api";
import Button from "../../shared/components/Button";
import Switch from "../../shared/components/Switch";
import { SettingRow } from "../../shared/components/ui";
import { errorMessage } from "../../shared/errors";
import { useLoaded } from "../../shared/hooks";
import { useT } from "../../shared/i18n";
import { toast } from "../../shared/toast";

/** Turns on the address overlays and stream decks read, and gives it to the person. */
export default function LocalApiSetting() {
  const { t } = useT();
  const saved = useLoaded(getLocalApi);
  const address = saved.data ? `http://127.0.0.1:${saved.data.port}/overlay` : "";

  async function change(on: boolean) {
    try {
      await setLocalApi(on);
      saved.reload();
    } catch (error) {
      toast(t("settings.localApiFailed"), { description: errorMessage(error) });
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(address);
    toast(t("settings.localApiCopied"));
  }

  return (
    <>
      <SettingRow title={t("settings.localApi")} hint={t("settings.localApiHint")}>
        <Switch checked={saved.data?.on ?? false} onChange={change} label={t("settings.localApi")} />
      </SettingRow>
      {saved.data?.on && (
        <SettingRow title={t("settings.localApiAddress")} hint={t("settings.localApiAddressHint")}>
          <div className="flex items-center gap-2">
            <code className="text-xs text-neutral-300">{address}</code>
            <Button variant="neutral" className="text-xs" onClick={copy}>
              {t("settings.localApiCopy")}
            </Button>
          </div>
        </SettingRow>
      )}
    </>
  );
}
