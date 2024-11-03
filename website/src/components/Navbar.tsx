"use client";
import { useState, type FC } from "react";
import type { NavbarProps } from "@/interfaces.d";
import { SectionsEnum } from "@/enums.d";
import Link from "next/link";

const Navbar: FC<NavbarProps> = ({ className }) => {
  const [activeSection, setActiveSection] = useState<string>();

  const toggleNavClick = (section: string) => setActiveSection(section);

  return (
    <nav>
      <ul className={`flex items-center gap-10 lg:gap-12 ${className}`}>
        {Object.values(SectionsEnum).map((section, index) => (
          <li
            key={index}
            className={`cursor-pointer hover:text-green-1 ${
              activeSection === section.toLowerCase() ? "text-green-1" : ""
            }`}
          >
            <Link
              href={`/#${section.toLowerCase()}`}
              onClick={() => toggleNavClick(section.toLowerCase())}
            >
              {section.split("_").join(" ")}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
};

export default Navbar;
