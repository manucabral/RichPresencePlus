import { Link } from "../../shared/router";
import { useT } from "../../shared/i18n";

/** The two halves of Presences: what you have, and what you can add. */
export default function PresenceTabs({ current }: { current: "mine" | "add" }) {
  const { t } = useT();
  const tabs = [
    { id: "mine", label: t("presences.tabMine"), to: "/presences" },
    { id: "add", label: t("presences.tabAdd"), to: "/catalog" },
  ] as const;
  return (
    <nav className="mb-5 flex gap-1 border-b border-neutral-800" aria-label={t("presences.title")}>
      {tabs.map((tab) => (
        <Link
          key={tab.id}
          to={tab.to}
          aria-current={tab.id === current ? "page" : undefined}
          className={`-mb-px border-b-2 px-3 py-2 text-[13px] ${
            tab.id === current
              ? "border-accent text-neutral-100"
              : "border-transparent text-neutral-400 hover:text-neutral-200"
          }`}
        >
          {tab.label}
        </Link>
      ))}
    </nav>
  );
}
