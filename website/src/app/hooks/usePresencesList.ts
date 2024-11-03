"use client";
import { Presences } from "@/interfaces";
import axios from "axios";
import PRESENCES_MOCK from "@/utils/PRESENCES_MOCK.json";
import { useEffect, useRef, useState } from "react";
import { getPresencesService } from "../services/presences";

const usePresencesList = ({ category }: { category: string }) => {
  const [presences, setPresences] = useState<Presences>([]);
  const [status, setStatus] = useState<"LOADING" | "ERROR" | "SUCCESS">(
    "LOADING",
  );
  const allPresences = useRef<Presences>([]);

  const formatPresencesMetadata = async (githubResponse: any) => {
    const subfolders = githubResponse.data.filter(
      (item: { type: string }) => item.type === "dir",
    );
    const metadataPromises = subfolders.map(
      async (folder: { name: string }) => {
        const metadataResponse = await axios.get(
          `https://api.github.com/repos/manucabral/richpresenceplus/contents/presences/${folder.name}/metadata.json`,
          {
            headers: {
              Authorization: `Bearer ${process.env.NEXT_PUBLIC_GITHUB_API_KEY}`,
            },
          },
        );
        const binaryString = window.atob(metadataResponse.data.content);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }
        const decodedText = new TextDecoder("utf-8").decode(bytes);
        const metadataContent = JSON.parse(decodedText);
        return metadataContent;
      },
    );
    return await Promise.all(metadataPromises);
  };

  const combinePresences = ({
    presences,
    mockPresences,
  }: {
    presences: Presences;
    mockPresences: any;
  }) => [...presences, ...mockPresences];

  useEffect(() => {
    const getPresences = async () => {
      try {
        const response = await getPresencesService();
        const presencesData = await formatPresencesMetadata(response);
        const combinedPresences = combinePresences({
          presences: presencesData,
          mockPresences: PRESENCES_MOCK,
        });
        setPresences(combinedPresences);
        allPresences.current = combinedPresences;
        setStatus("SUCCESS");
      } catch (error) {
        console.error("Error getting presences:", error);
        setStatus("ERROR");
      }
    };
    getPresences();
  }, []);

  useEffect(() => {
    const filterPresences = () => {
      if (category === "all") return allPresences.current;
      const filteredPresences = allPresences.current.filter(
        (presence) => presence.category.toLowerCase() === category,
      );
      return filteredPresences;
    };
    const filteredPresences = filterPresences();
    setPresences(filteredPresences);
  }, [category]);

  return { presences, status };
};

export default usePresencesList;
