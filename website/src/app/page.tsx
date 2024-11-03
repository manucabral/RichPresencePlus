import Download from "@/sections/Download";
import Hero from "@/sections/Hero";
import FAQ from "@/sections/FAQ";
import type { FC } from "react";
import Presences from "@/sections/Presences";
import CreatePresences from "@/sections/CreatePresences";

const Home: FC = () => {
  return (
    <main className="relative flex h-max w-full flex-col bg-dark px-5 pt-20 text-white md:gap-0 md:pt-0">
      <Hero />
      <Presences />
      <CreatePresences />
      <FAQ />
      <Download />
    </main>
  );
};

export default Home;
