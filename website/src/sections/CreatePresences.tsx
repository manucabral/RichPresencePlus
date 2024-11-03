import Section from "@/components/Section";
import { SectionsEnum } from "@/enums.d";
import { SectionsInfo } from "@/constants/sectionsInfo";
import Button from "@/components/Button";

export default function CreatePresences() {
  return (
    <Section className="mt-16 !h-full">
      <Section.Container
        id={SectionsEnum?.CREATE_PRESENCES?.toLowerCase()}
        offset={-400}
      >
        <Section.ContentContainer className="!flex-auto">
          <Section.Title
            className="!text-center !text-3xl"
            title={SectionsInfo.CREATE_PRESENCES.TITLE}
          />
          <div className="flex flex-col items-center justify-center gap-10">
            <p className="max-w-[1000px] text-center text-gray-300">
              {SectionsInfo.CREATE_PRESENCES.DESCRIPTION}
            </p>
            <Button
              variant="transparent"
              type="link"
              href="https://github.com/manucabral/RichPresencePlusDev"
              target="_blank"
            >
              Get started
            </Button>
          </div>
        </Section.ContentContainer>
      </Section.Container>
    </Section>
  );
}
