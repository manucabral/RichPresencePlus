// YouTube Music: shows the song that is playing.
// The browser extension reads what the site publishes through the Media Session
// API (title, artist, album, cover); see manifest.json.

const LOGO = "youtube_music_logo";
const MAX_IMAGE = 512; // Discord refuses a longer image address.

function read(context) {
  // Only tabs that are playing something count.
  const page = bestPage(context.pages.filter((p) => p.values.session));
  if (!page) return null;

  const song = page.values.session;
  const clock = playback(page.values.player, page.received_at);
  return {
    title: song.title,
    artist: song.artist,
    album: context.options.show_album ? song.album : null,
    artwork: song.artwork,
    // The site says whether it is playing; the player is the fallback.
    playing: song.state ? song.state === "playing" : clock.playing,
    start: clock.start,
    end: clock.end,
  };
}

function activity(song) {
  const artistAndAlbum = [song.artist, song.album].filter(Boolean).join(" | ");
  const cover = song.artwork && song.artwork.length <= MAX_IMAGE ? song.artwork : LOGO;

  return {
    details: cut(song.title) || t("listening"),
    state: cut(artistAndAlbum),
    kind: "listening",
    start: song.playing ? song.start : null,
    end: song.playing ? song.end : null,
    large_image: cover,
    large_text: "YouTube Music",
    small_image: song.playing ? "playing" : "pause",
    small_text: song.playing ? t("playing") : t("paused"),
  };
}

// A paused song gives way when something else is playing.
function meta(song) {
  return { paused: !song.playing };
}
