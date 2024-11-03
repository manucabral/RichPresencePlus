import Section from "@/components/Section";
import { SectionsInfo } from "@/constants/sectionsInfo";
import { SectionsEnum } from "@/enums.d";
import type { FC } from "react";
import EasyInterfaceIcon from "/public/images/easy-interface-icon.svg";
import SwitchButtonIcon from "/public/images/switch-button-icon.svg";
import InternetIcon from "/public/images/internet-icon.svg";
import BoltIcon from "/public/images/bolt-icon.svg";

const features = {
  easyInterface: {
    image: <EasyInterfaceIcon className="h-10 w-10" />,
    title: "Easy Interface",
    description:
      "Our interface is easy to use and intuitive. You can easily manage your presences.",
  },
  switchButton: {
    image: <SwitchButtonIcon className="h-10 w-10" />,
    title: "Switch presences",
    description: "Switch between presences with a single click.",
  },
  internet: {
    image: <InternetIcon className="h-10 w-10" />,
    title: "A world of presences",
    description:
      "You can use any presence you want. Check out our community presences or create your own.",
  },
  bolt: {
    image: <BoltIcon className="h-10 w-10" />,
    title: "Fast connection",
    description:
      "Rich Presence Plus is fast and reliable. You will not have any problems with your presences.",
  },
};

const Features: FC = () => {
  return (
    <ul className="grid w-full max-w-layout grid-cols-2 justify-center gap-10 md:flex">
      {Object.entries(features).map(([key, value]) => (
        <li key={key} className="flex flex-1 flex-col items-center gap-1">
          {value.image}
          <h3 className="text-center text-base lg:text-lg">{value.title}</h3>
          <p className="text-center text-sm text-gray-400 lg:text-base">
            {value.description}
          </p>
        </li>
      ))}
    </ul>
  );
};

export default Features;
