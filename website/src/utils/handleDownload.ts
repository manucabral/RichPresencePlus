export const handleDownload = () => {
  const downloadUrl = "/files/Rich.Presence.Plus.v0.0.5.zip";
  const link = document.createElement("a");
  link.href = downloadUrl;
  link.download = "Rich.Presence.Plus.v0.0.5.zip";
  link.click();
};
