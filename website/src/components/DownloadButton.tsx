"use client";
import useDownload from "@/app/hooks/useDownload";
import Button from "./Button";

const DownloadButton = ({ className }: { className?: string }) => {
  const { handleDownload } = useDownload();

  return (
    <Button
      className={className}
      variant="green"
      type="button"
      onClick={() => handleDownload()}
    >
      Download
    </Button>
  );
};

export default DownloadButton;
