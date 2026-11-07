import { toast } from "../toast";
import { openLink } from "../../platform/api";
import { errorMessage } from "../errors";
import { useT } from "../i18n";
import { linkParts } from "../links";

/** A text whose known addresses open in the browser. */
export default function Linked({ text }: { text: string }) {
  const { t } = useT();

  async function open(event: Event, target: string) {
    // It can sit inside a label, whose click would move the focus to the field.
    event.preventDefault();
    try {
      await openLink(target);
    } catch (error) {
      toast(t("legal.failed"), { description: errorMessage(error) });
    }
  }

  return (
    <>
      {linkParts(text).map((part) =>
        part.target === null ? (
          part.text
        ) : (
          <button
            key={part.text}
            type="button"
            style={{ minHeight: 0 }}
            className="text-accent-soft underline underline-offset-2 hover:text-accent-hover"
            onClick={(event) => open(event, part.target as string)}
          >
            {part.text}
          </button>
        ),
      )}
    </>
  );
}
