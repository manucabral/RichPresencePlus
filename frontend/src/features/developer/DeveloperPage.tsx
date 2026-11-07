import { useState } from "preact/hooks";
import InfoCard from "../../shared/components/InfoCard";
import PageTitle from "../../shared/components/PageTitle";
import Tabs from "../../shared/components/Tabs";
import { pageWidth } from "../../shared/components/ui";
import { useT } from "../../shared/i18n";
import CasesPanel from "./CasesPanel";
import CodePanel from "./CodePanel";
import CreateCard from "./CreateCard";
import LiveCard from "./LiveCard";
import OwnPanel from "./OwnPanel";
import PublishPanel from "./PublishPanel";
import { usePick } from "./usePick";
import { firstTab, type DeveloperTab } from "./workbench";

export default function DeveloperPage() {
  const { t } = useT();
  const pick = usePick();
  const [chosen, setChosen] = useState<DeveloperTab | null>(null);
  // Saving a case in the live view adds a row to the list under it.
  const [casesSaved, setCasesSaved] = useState(0);
  const tab = chosen ?? firstTab(pick.mine.length > 0, pick.linked);

  const tabs: { key: DeveloperTab; label: string }[] = [
    { key: "create", label: t("developer.tabCreate") },
    { key: "code", label: t("developer.tabCode") },
    { key: "test", label: t("developer.tabTest") },
    { key: "publish", label: t("developer.tabPublish") },
  ];

  function created(name: string) {
    pick.setPicked(name);
    pick.reload();
    setChosen("code");
  }

  const presence = pick.mine.find((item) => item.name === pick.own);

  return (
    <div className={`${pageWidth} space-y-4`}>
      <PageTitle title={t("developer.title")} subtitle={t("developer.subtitle")} />
      <Tabs tabs={tabs} active={tab} label={t("developer.title")} onChange={setChosen} />
      {tab === "create" && <CreateCard onCreated={created} />}
      {tab === "code" && (
        <OwnPanel pick={pick}>{() => presence && <CodePanel presence={presence} onChanged={pick.reload} />}</OwnPanel>
      )}
      {tab === "test" && (
        <>
          <LiveCard pick={pick} onCaseSaved={() => setCasesSaved((count) => count + 1)} />
          {pick.canWatch && (
            <section className="rounded-lg border border-neutral-800 bg-neutral-900/40 p-4">
              <CasesPanel key={`${pick.name}:${casesSaved}`} name={pick.name} />
            </section>
          )}
        </>
      )}
      {tab === "publish" && <OwnPanel pick={pick}>{(name) => <PublishPanel key={name} name={name} />}</OwnPanel>}
      <InfoCard text={t("developer.docs")} />
    </div>
  );
}
