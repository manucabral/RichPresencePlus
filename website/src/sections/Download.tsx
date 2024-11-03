import Image from "next/image";
import Section from "@/components/Section";
import { SectionsInfo } from "@/constants/sectionsInfo";
import type { FC } from "react";
import horizontalLinesImage from "/public/images/horizontal-lines.png";
import DownloadButton from "@/components/DownloadButton";

const Download: FC = () => {
  return (
    <Section className="relative mb-16 !h-max">
      <Section.Container
        className="justify-center"
        id={"download"}
        offset={-500}
      >
        <Section.ContentContainer>
          <Section.Title
            className="whitespace-pre-wrap !text-center !text-2xl lg:!text-4xl"
            title={SectionsInfo.DOWNLOAD.TITLE}
          />
          <div className="flex w-full justify-center">
            <DownloadButton />
          </div>
        </Section.ContentContainer>
      </Section.Container>
      <Image
        src={horizontalLinesImage}
        className="pointer-events-none absolute hidden lg:inline-block"
        alt=""
        unoptimized
      />
    </Section>
  );
};

export default Download;
