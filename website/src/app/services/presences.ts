import axios from "axios";

export const getPresencesService = async () => {
  const response = await axios.get(
    `https://api.github.com/repos/manucabral/richpresenceplus/contents/presences`,
  );
  return response;
};
