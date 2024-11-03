"use client";
import Navbar from "./Navbar";
import { useState, type FC } from "react";
import { Bars4Icon } from "@heroicons/react/24/solid";
import Logo from "./Logo";
import Button from "./Button";
import DownloadButton from "./DownloadButton";

const Header: FC = () => {
  const [showResponsiveNavbar, setShowResponsiveNavbar] = useState(false);

  const handleShowResponsiveNavbar = () =>
    setShowResponsiveNavbar(!showResponsiveNavbar);

  return (
    <header className="backdrop-contrast-00 fixed inset-0 z-10 flex h-max w-full justify-center border-b-[.5px] border-b-gray-700 bg-[#1815268b] p-5 text-white backdrop-blur-sm backdrop-filter">
      <section className="flex h-max w-full max-w-layout items-center justify-between">
        <div className="flex items-center gap-5">
          <Logo />
        </div>
        <button className="lg:hidden" onClick={handleShowResponsiveNavbar}>
          <Bars4Icon className="h-6 w-6" />
        </button>
        <div
          className={`${
            showResponsiveNavbar ? "top-[76px]" : "top-[-400px]"
          } absolute inset-0 flex h-max w-full flex-col justify-center gap-5 border border-gray-700 bg-[#211d32] p-5 duration-500 lg:hidden`}
        >
          <Navbar className="flex-col gap-10 lg:flex-row" />
          <div className="flex flex-col items-center gap-5">
            <Button
              variant="transparent"
              type="link"
              href={"https://github.com/manucabral/RichPresencePlus"}
              className="!px-5 !py-2"
              target="_blank"
            >
              Github
            </Button>
            <DownloadButton className="!px-5 !py-2" />
          </div>
        </div>
        <div className="hidden lg:block">
          <Navbar />
        </div>
        <div className="hidden gap-5 lg:flex">
          <Button
            variant="transparent"
            type="link"
            href={"https://github.com/manucabral/RichPresencePlus"}
            className="!px-5 !py-2"
            target="_blank"
          >
            Github
          </Button>
          <DownloadButton className="!px-5 !py-2" />
        </div>
      </section>
    </header>
  );
};

export default Header;
