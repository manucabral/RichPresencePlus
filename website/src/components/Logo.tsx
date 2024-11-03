import type { LogoProps } from "@/interfaces";
import Image from "next/image";
import Link from "next/link";
import type { FC } from "react";

const Logo: FC<LogoProps> = ({ containerClassName, logoClassName }) => {
  return (
    <Link
      href={"/"}
      className={`flex items-center gap-3 ${containerClassName}`}
    >
      <Image
        src="/images/rich-presence-plus-logo.png"
        alt="Rich Presence Plus logo"
        className={logoClassName}
        width={40}
        height={40}
        unoptimized
      />
      <h4>Rich Presence Plus</h4>
    </Link>
  );
};

export default Logo;
