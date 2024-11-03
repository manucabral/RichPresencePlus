import type { FC } from "react";
import type { ButtonProps, Variants } from "../interfaces.d";
import Link from "next/link";

const variants: Variants = {
  green: "bg-green-1 hover:bg-green-4 hover:text-green-18",
  transparent:
    "bg-transparent border border-green-1 hover:border-green-1 hover:!text-green-1 !text-white",
};

const defaultClassName =
  "w-max flex gap-3 lg:gap-5 font-bold focus:outline-none text-black cursor-pointer flex justify-center items-center text-base lg:text-base px-7 py-[8px] lg:px-10 lg:py-4 box-border duration-75 text-center rounded-md text-black";

const Button: FC<ButtonProps> = ({
  variant,
  type,
  href,
  className,
  onClick,
  children,
  disabled,
  target,
}) => {
  return type === "link" ? (
    <Link
      className={`${
        variants[variant as keyof Variants]
      } ${defaultClassName} ${className}`}
      href={href!}
      target={target === "_blank" ? "_blank" : undefined}
    >
      {children}
    </Link>
  ) : (
    <button
      className={`${
        variants[variant as keyof Variants]
      } ${defaultClassName} ${className}`}
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </button>
  );
};

export default Button;
