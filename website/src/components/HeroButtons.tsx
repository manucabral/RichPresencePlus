"use client";
import DownloadButton from "@/components/DownloadButton";
import Button from "@/components/Button";
import type { FC } from "react";

const HeroButtons: FC = () => {
  return (
    <div className="flex justify-center gap-5 md:justify-start">
      <DownloadButton />
      <Button variant="transparent" type="link" href="#presences">
        Presences List
      </Button>
    </div>
  );
};

export default HeroButtons;
