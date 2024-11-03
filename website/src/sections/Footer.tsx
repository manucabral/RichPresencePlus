import Logo from "@/components/Logo";
import Link from "next/link";
import type { FC } from "react";

const Footer: FC = () => {
  return (
    <footer className="flex h-max w-full flex-col items-center justify-center bg-dark text-white">
      <div className="flex h-[100px] w-full justify-center">
        <div className="flex w-full max-w-[1200px] flex-col gap-3 border-t border-t-gray-700 px-5 py-3 sm:flex-row sm:items-center sm:justify-between xl:px-0">
          <div className="flex items-center gap-5">
            <Logo titleClassName="!text-base" />
          </div>
          <div className="flex flex-wrap gap-5 text-xs sm:text-base">
            <Link
              href={"https://github.com/manucabral/RichPresencePlus"}
              target="_blank"
            >
              Github
            </Link>
            <Link href={"/privacy-policy"}>Privacy policy</Link>
            <Link href={"/terms-and-conditions"}>Terms & conditions</Link>
            <Link href={"/about-us"}>About us</Link>
          </div>
        </div>
      </div>
      <div className="flex h-16 w-full items-center justify-center bg-[#0f081d] px-3 text-white xl:px-0">
        <span className="text-center text-xs md:text-sm">
          ® 2024 - Rich Presence Plus. All rights reserved.
        </span>
      </div>
    </footer>
  );
};

export default Footer;
