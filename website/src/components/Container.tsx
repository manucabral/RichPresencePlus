"use client";
import { ContainerProps } from "@/interfaces";
import type { FC } from "react";
import { useInView } from "react-intersection-observer";

export const Container: FC<ContainerProps> = ({
  className,
  inverted,
  children,
  id,
  offset,
}) => {
  const { ref, inView } = useInView({
    triggerOnce: true,
    threshold: 0.3,
  });
  return (
    <div
      className={`relative flex h-max w-full max-w-[1200px] flex-wrap-reverse items-center gap-x-20 gap-y-10 md:flex-nowrap ${className} ${
        inverted ? "flex-row-reverse" : "flex-row"
      } transition-opacity duration-1000 ${inView ? "opacity-1" : "opacity-0"}`}
      ref={ref}
    >
      <div
        className={`absolute inset-0 h-full w-full ${className} section-container pointer-events-none`}
        id={id}
        style={{
          top: `${offset}px`,
          height: `calc(100% - ${offset}px)`,
        }}
      ></div>
      {children}
    </div>
  );
};
