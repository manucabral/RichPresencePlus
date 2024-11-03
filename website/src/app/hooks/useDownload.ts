import { useEffect, useState } from "react";
import { getReleasesService } from "@/app/services/releases";

const useDownload = () => {
  const [downloadUrl, setDownloadUrl] = useState(null);

  useEffect(() => {
    const fetchLatestRelease = async () => {
      try {
        const response = await getReleasesService();
        const release = response.data;
        const zipAsset = release.assets.find((asset: { name: string }) =>
          asset.name.endsWith(".zip"),
        );
        if (zipAsset) {
          setDownloadUrl(zipAsset.browser_download_url);
        }
      } catch (error) {
        console.error("Failed to get the latest release:", error);
      }
    };
    fetchLatestRelease();
  }, []);

  const handleDownload = () => {
    if (downloadUrl) {
      const link = document.createElement("a");
      link.href = downloadUrl;
      link.download = "Rich.Presence.Plus.zip";
      link.click();
    } else {
      console.error("Url not found. Please try again later.");
    }
  };
  return { handleDownload };
};

export default useDownload;
