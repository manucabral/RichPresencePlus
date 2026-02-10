/**
 * Netflix Metadata Extractor
 * 
 * Extracts video metadata from Netflix using the internal API.
 * Must be executed in the browser context to access session cookies.
 * 
 * @returns {Object|null} Video data object or null if no video found
 * @returns {Object} Error object if extraction fails
 */
(async () => {
  const video = document.querySelector("video");
  if (!video) return null;
  
  const urlMatch = window.location.href.match(/\/watch\/(\d+)/);
  if (!urlMatch) return { error: 'No media ID in URL' };
  
  try {
    const controller = new AbortController();
    setTimeout(() => controller.abort(), 6000);
    const res = await fetch(
      `https://www.netflix.com/nq/website/memberapi/release/metadata?movieid=${urlMatch[1]}`,
      { signal: controller.signal, credentials: 'include' }
    );
    if (!res.ok) return { error: `API ${res.status}` };
    const { video: v } = await res.json();
    if (!v) return { error: 'No video data' };
    const result = {
      currentTime: video.currentTime,
      duration: video.duration,
      paused: video.paused,
      type: v.type,
      title: v.title,
      artwork: v.artwork?.[0]?.url || null,
      episode: null,
      season: null,
      episodeTitle: null
    };
    if (v.type === 'show' && v.currentEpisode && v.seasons) {
      for (const s of v.seasons) {
        const ep = s.episodes?.find(e => e.id === v.currentEpisode);
        if (ep) {
          result.season = s.seq;
          result.episode = ep.seq;
          result.episodeTitle = ep.title;
          break;
        }
      }
    }
    
    return result;
  } catch (error) {
    return { error: error.message };
  }
})();
