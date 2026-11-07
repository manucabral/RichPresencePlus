import { toast } from "../../shared/toast";
import { openLegal } from "../../platform/api";
import Button from "../../shared/components/Button";
import { errorMessage } from "../../shared/errors";
import { current, useT } from "../../shared/i18n";

const PAGES = ["privacy", "terms", "licenses"] as const;

/** Links to the legal pages, which open in the browser. */
export default function LegalSetting() {
  const { t } = useT();

  async function open(page: (typeof PAGES)[number]) {
    try {
      await openLegal(page, current());
    } catch (error) {
      toast(t("legal.failed"), { description: errorMessage(error) });
    }
  }

  return (
    <div className="flex flex-wrap gap-2 px-4 py-3">
      {PAGES.map((page) => (
        <Button key={page} variant="ghost" className="text-xs" onClick={() => open(page)}>
          {t(`legal.${page}`)}
        </Button>
      ))}
    </div>
  );
}
