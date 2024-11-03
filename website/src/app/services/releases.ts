import axios from "axios";

export const getReleasesService = async () => {
  const response = await axios.get(
    "https://api.github.com/repos/manucabral/richpresenceplus/releases/latest",
    {
      headers: {
        Authorization: `Bearer ${process.env.NEXT_PUBLIC_GITHUB_API_KEY}`,
      },
    },
  );
  return response;
};
