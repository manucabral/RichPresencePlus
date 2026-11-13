// Stremio: shows the movie or episode playing on web.stremio.com.
// The browser extension reads the page; what it reads is named in manifest.json.

const NAME = "Stremio";
// The presence's own icon, for when there is no poster.
const ICON = "https://raw.githubusercontent.com/manucabral/RichPresencePlus/main/presences/stremio/icon.webp";

// The player's address ends in /<type>/<id>/<video id>, after the "#".
// A series episode's video id is "tt123:season:episode".
function where(address) {
  const hash = String(address || "").split("#")[1] || "";
  const parts = hash.split("/").filter(Boolean);
  if (parts[0] !== "player") return null;
  const [type, id, video] = parts.slice(-3).map((part) => {
    try {
      return decodeURIComponent(part);
    } catch {
      return part;
    }
  });
  if (!/^tt\d+$/.test(id || "")) return { type: null, id: null };
  const episode = /^tt\d+:(\d+):(\d+)$/.exec(video || "");
  return {
    type,
    id,
    season: episode ? Number(episode[1]) : null,
    episode: episode ? Number(episode[2]) : null,
  };
}

// "01:02:03" or "02:03" in seconds; anything else gives null.
function seconds(text) {
  const parts = String(text || "")
    .trim()
    .split(":")
    .map(Number);
  if (parts.length < 2 || parts.some((part) => !Number.isFinite(part))) return null;
  return parts.reduce((total, part) => total * 60 + part, 0);
}

function read(context) {
  // Streams in a frame leave no <video> on the page, so a tab on the player
  // is preferred over the others before the usual choice.
  const pages = context.pages || [];
  const watching = pages.filter((item) => where(item.url) && item.values?.title);
  const page = bestPage(watching.length ? watching : pages);
  if (!page) return null;

  const values = page.values;
  const place = where(page.url);
  if (!place || !values.title) return { browsing: true };

  // Streams from a file play in a <video>; others in a frame the page drives
  // with its own controls, so their clock and state come from those controls.
  let player = values.player;
  if (!player) {
    const current = seconds(values.current);
    const duration = seconds(values.duration);
    player =
      current === null ? null : { current, duration: duration || null, rate: 1, paused: values.button !== "Pause" };
  }
  return {
    browsing: false,
    title: values.title,
    id: place.id,
    type: place.type,
    season: place.season,
    episode: place.episode,
    ...playback(player, page.received_at),
  };
}

function activity(show) {
  if (show.browsing) {
    return { name: NAME, details: t("browsing"), kind: "watching", large_image: ICON, large_text: NAME };
  }

  const state = show.season !== null ? t("episode", { season: show.season, episode: show.episode }) : null;
  const buttons = show.id
    ? [{ label: t("open"), url: `https://web.stremio.com/#/detail/${show.type}/${show.id}` }]
    : [];

  return {
    name: NAME,
    details: cut(show.title) || t("watching"),
    state,
    kind: "watching",
    start: show.start,
    end: show.end,
    large_image: show.id ? `https://images.metahub.space/poster/medium/${show.id}/img` : ICON,
    large_text: show.title || NAME,
    small_text: show.playing ? t("playing") : t("paused"),
    buttons,
  };
}

// A paused video gives way when something else is playing.
function meta(show) {
  return { paused: !show.browsing && !show.playing };
}
