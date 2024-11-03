import Section from "@/components/Section";
import { QUESTION_ANSWERS } from "@/constants/questionsAswers";
import { SectionsInfo } from "@/constants/sectionsInfo";
import { SectionsEnum } from "@/enums.d";
import type { FC } from "react";

const FAQ: FC = () => {
  return (
    <Section className="!h-full py-16 md:py-32">
      <Section.Container id={SectionsEnum?.FAQ?.toLowerCase()} offset={-100}>
        <Section.ContentContainer className="!flex-auto">
          <Section.Title
            className="!text-center !text-3xl"
            title={SectionsInfo.FAQ.TITLE}
          />
          <div className="mx-auto w-full max-w-7xl items-center text-white">
            <div className="mx-auto w-full text-left">
              <div className="relative m-auto items-center gap-12 md:order-first lg:inline-flex">
                <div className="mx-auto lg:max-w-7xl lg:p-0">
                  <ul
                    role="list"
                    className="grid list-none grid-cols-2 gap-4 lg:grid-cols-3 lg:gap-12"
                  >
                    {QUESTION_ANSWERS.map(({ QUESTION, ANSWER }) => (
                      <li key={QUESTION}>
                        <div>
                          <p className="mt-5 text-lg font-medium leading-6 text-green-1">
                            {QUESTION}
                          </p>
                        </div>
                        <div className="mt-2 text-base text-gray-300">
                          {ANSWER}
                        </div>
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            </div>
          </div>
        </Section.ContentContainer>
      </Section.Container>
    </Section>
  );
};

export default FAQ;
