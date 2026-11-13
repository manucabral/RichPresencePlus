// Kick: shows the stream or the video being watched.
// The browser extension reads the page; what it reads is named in manifest.json.

const NAME = "Kick";
// The presence's own icon, for when the page gives no picture.
const ICON = "https://raw.githubusercontent.com/manucabral/RichPresencePlus/main/presences/kick/icon.webp";

// First parts of an address that are Kick's own pages, not a channel.
const PAGES = new Set([
  "browse",
  "category",
  "categories",
  "search",
  "following",
  "settings",
  "dashboard",
  "subscriptions",
  "messages",
  "terms-of-service",
  "privacy-policy",
  "popout",
  "stream",
  "auth",
  "login",
  "signup",
  "wallet",
  "transactions",
]);

function where(url) {
  const parts = url.path.split("/").filter(Boolean);
  if (!parts.length || PAGES.has(parts[0].toLowerCase())) return null;
  // Only a channel and one of its videos count; its chat, clips or tabs do not.
  if (parts.length === 1) return { channel: parts[0], video: null };
  if (parts.length === 3 && parts[1] === "videos") return { channel: parts[0], video: parts[2] };
  return null;
}

function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;

  const values = page.values;
  const place = where(parseUrl(page.url));
  if (!place || !values.player) return { browsing: true };

  // A live stream has no fixed length, so it gets no clock.
  const player = values.player;
  const live = !place.video && !(player.duration > 0 && Number.isFinite(player.duration));
  const clock = live ? { playing: !player.paused, start: null, end: null } : playback(player, page.received_at);
  return {
    browsing: false,
    live,
    video: place.video,
    login: place.channel,
    title: values.title,
    channel: values.channel || place.channel,
    category: values.category,
    image: values.image,
    ...clock,
  };
}

function activity(stream) {
  if (stream.browsing) {
    return { name: NAME, details: t("browsing"), kind: "watching", large_image: ICON, large_text: NAME };
  }

  const url = stream.video
    ? `https://kick.com/${stream.login}/videos/${stream.video}`
    : `https://kick.com/${stream.login}`;
  const parts = [stream.channel, stream.category].filter(Boolean).join(" · ");

  return {
    name: NAME,
    details: cut(stream.title) || cut(stream.channel) || t("watching"),
    state: parts ? cut(parts) : null,
    kind: "watching",
    start: stream.start,
    end: stream.end,
    large_image: stream.image?.startsWith("https://") ? stream.image : ICON,
    large_text: stream.channel || NAME,
    small_text: stream.live ? t("live") : stream.playing ? t("playing") : t("paused"),
    buttons: [{ label: stream.live ? t("watch_live") : t("watch"), url }],
  };
}

// A paused video gives way when something else is playing.
function meta(stream) {
  return { paused: !stream.browsing && !stream.playing };
}
