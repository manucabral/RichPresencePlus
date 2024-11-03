import type {
  ContentContainerProps,
  DescriptionProps,
  ExtendedSection,
  ImageProps,
  SectionProps,
  TitleProps,
} from "@/interfaces";
import type { FC } from "react";
import Image from "next/image";
import { Container } from "./Container";

const Section: FC<SectionProps> & ExtendedSection = ({
  className,
  children,
}) => {
  return (
    <section
      className={`relative flex h-max items-center justify-center md:h-screen ${className}`}
    >
      {children}
    </section>
  );
};

const ContentContainer: FC<ContentContainerProps> = ({
  children,
  className,
}) => (
  <div className={`flex w-full flex-col gap-10 lg:w-2/4 ${className}`}>
    {children}
  </div>
);

const Title: FC<TitleProps> = ({ className, title, inverted }) => (
  <h2
    className={`text-center text-3xl font-bold md:text-start lg:text-6xl ${className} ${
      inverted ? "text-right" : "text-left"
    }`}
  >
    {title}
  </h2>
);

const Description: FC<DescriptionProps> = ({ className, description }) => (
  <p className={`text-base text-gray-400 lg:text-lg ${className}`}>
    {description}
  </p>
);

const Image_: FC<ImageProps> = ({ image }) => {
  return (
    <div className="w-[400px]">
      <Image
        src={image}
        alt="image"
        className="w-full object-cover"
        unoptimized
      />
    </div>
  );
};

Section.Container = Container;
Section.ContentContainer = ContentContainer;
Section.Title = Title;
Section.Description = Description;
Section.Image = Image_;

export default Section;
