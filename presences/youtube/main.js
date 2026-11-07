// YouTube: shows the video being watched.
// The browser extension reads the page; what it reads is named in manifest.json.

const LOGO = "youtube_logo";

// The video id is in `?v=` on watch pages and in the path on Shorts.
function videoId(url) {
  if (url.query.v) return url.query.v;
  const short = /^\/shorts\/([^/]+)/.exec(url.path);
  return short ? short[1] : null;
}

function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;

  const url = parseUrl(page.url);
  const values = page.values;
  const isShort = url.path.startsWith("/shorts/");
  return {
    id: videoId(url),
    browsing: url.path === "/" || !values.player,
    title: isShort ? values.shorts_title : values.title || page.title,
    author: isShort ? values.shorts_author : values.author,
    author_url: values.author_url,
    thumbnail: context.options.image !== "logo",
    ...playback(values.player, page.received_at),
  };
}

function activity(video) {
  if (video.browsing) {
    return { details: t("browsing"), kind: "watching", large_image: LOGO, large_text: "YouTube" };
  }

  const buttons = [];
  if (video.id) buttons.push({ label: t("watch"), url: `https://www.youtube.com/watch?v=${video.id}` });
  if (video.author_url) buttons.push({ label: t("author"), url: video.author_url });

  return {
    details: cut(video.title) || t("watching"),
    state: video.author ? cut(t("by", { author: video.author })) : null,
    kind: "watching",
    start: video.start,
    end: video.end,
    large_image: video.id && video.thumbnail ? `https://i.ytimg.com/vi/${video.id}/hqdefault.jpg` : LOGO,
    large_text: "YouTube",
    small_image: video.playing ? "play" : "pause",
    small_text: video.playing ? t("playing") : t("paused"),
    buttons,
  };
}

// A paused video gives way when something else is playing.
function meta(video) {
  return { paused: !video.browsing && !video.playing };
}
