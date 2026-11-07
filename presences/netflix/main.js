// Netflix: shows the title being watched.
// The browser extension reads the player page; see manifest.json.

function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;

  const url = parseUrl(page.url);
  const values = page.values;
  return {
    // Only /watch/ pages have a title playing; everything else is browsing.
    browsing: !values.player || !url.path.startsWith("/watch/"),
    title: values.title,
    episode: values.episode,
    episode_title: values.episode_title,
    // Without the query string, which can carry tracking parameters.
    address: `https://${url.host}${url.path}`,
    ...playback(values.player, page.received_at),
  };
}

function activity(show) {
  if (show.browsing) {
    return { details: t("browsing"), state: t("looking"), kind: "watching" };
  }

  const episode = [show.episode, show.episode_title].filter(Boolean).join(" - ");
  return {
    details: cut(show.title) || t("watching"),
    state: cut(episode),
    kind: "watching",
    start: show.start,
    end: show.end,
    small_image: show.playing ? "play" : "pause",
    small_text: show.playing ? t("playing") : t("paused"),
    buttons: [{ label: t("watch"), url: show.address }],
  };
}

// A paused title gives way when something else is playing.
function meta(show) {
  return { paused: !show.browsing && !show.playing };
}
