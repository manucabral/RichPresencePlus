import HeroButtons from "@/components/HeroButtons";
import Section from "@/components/Section";
import type { FC } from "react";
import { SectionsEnum } from "@/enums.d";
import { SectionsInfo } from "@/constants/sectionsInfo";
import Image from "next/image";
import Features from "./Features";

const Hero: FC = () => {
  return (
    <Section className="mt-10 !h-full flex-col gap-16 md:mb-20 md:mt-52 md:gap-32">
      <Section.Container id={SectionsEnum.HOME.toLowerCase()} offset={-400}>
        <Section.ContentContainer className="!w-full !items-center md:!items-start">
          <div className="flex flex-wrap items-center justify-center gap-5 lg:justify-start">
            <div className="relative size-14 sm:size-16">
              <Image
                src="/images/rich-presence-plus-logo.svg"
                alt="Rich Presence Plus logo"
                fill
                unoptimized
              />
            </div>
            <h1 className="text-center text-3xl font-bold md:text-5xl">
              Rich Presence Plus
            </h1>
          </div>
          <Section.Description
            description={SectionsInfo.HOME.DESCRIPTION}
            className="max-w-3xl text-center text-gray-300 md:text-start"
          />
          <HeroButtons />
        </Section.ContentContainer>
        <div className="relative h-[400px] w-full">
          <Image
            src="/files/rich-presence-plus-demo.gif"
            alt="Rich Presence Plus demo"
            className="object-cover"
            fill
            unoptimized
          />
        </div>
      </Section.Container>
      <Features />
    </Section>
  );
};

export default Hero;
