import { Url } from "next/dist/shared/lib/router/router";
import { StaticImageData } from "next/image";
import { SectionsEnum } from "./enums";

export type Variants = {
  green: string;
  transparent: string;
};

export interface ButtonProps {
  variant: "green" | "transparent";
  type: "submit" | "button" | "link";
  href?: Url;
  className?: string;
  onClick?: () => void;
  children: React.ReactNode;
  disabled?: boolean;
  target?: string;
}

export interface NavbarProps {
  className?: string;
}

export interface LogoProps {
  containerClassName?: string;
  logoClassName?: string;
  titleClassName?: string;
}

export interface SectionProps {
  className?: string;
  children: React.ReactNode;
}

export interface ContainerProps {
  className?: string;
  children: React.ReactNode;
  inverted: boolean;
  id: SectionsEnum;
  offset: number;
}

export interface ContentContainerProps {
  className?: string;
  children: React.ReactNode;
}

export interface TitleProps {
  className?: string;
  title: string;
  inverted: boolean;
}

export interface DescriptionProps {
  className?: string;
  description: string;
}

export interface ImageProps {
  image: string;
}

export type ExtendedSection = {
  Container: FC<ContainerProps>;
} & {
  ContentContainer: FC<ContentContainerProps>;
} & {
  Title: FC<TitleProps>;
} & {
  Description: FC<DescriptionProps>;
} & {
  Image: FC<ImageProps>;
};

export interface refProps {
  current: HTMLDivElement | null;
}

export interface SectionRefs {
  [key: string]: RefObject<HTMLDivElement>;
}

export interface FilterOptionProps {
  value: string;
  label: string;
  checked: boolean;
}

export interface SortOptionsProps {
  name: string;
  value: string;
}

export interface SortOptionProps {
  option: {
    name: string;
    value: string;
  };
}

export interface MobileFilterSidebarProps {
  open: boolean;
  onClose: () => void;
  defaultFilter: FilterOptionProps["label"];
}

export interface PresencesHeaderProps {
  onClick: () => void;
}

export interface Presence {
  clientId: string;
  name: string;
  author: string;
  description: {
    en: string;
    es?: string;
    "pt-BR"?: string;
  };
  image: string;
  version: string;
  category: string;
  color: string;
  inProgress?: boolean;
}

export type Presences = Presence[];
