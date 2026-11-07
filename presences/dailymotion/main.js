// Dailymotion: shows the video being watched.
// The page gives the title and the channel; the player lives in a frame from
// geo.dailymotion.com, which gives the playback. See manifest.json.

const NAME = "Dailymotion";

// Video ids are an "x" and letters or digits. Old addresses add "_" and a slug.
function videoId(url) {
  const match = /^\/video\/(x[0-9a-z]+)/i.exec(url.path);
  return match ? match[1] : null;
}

function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;

  const values = page.values;
  const id = videoId(parseUrl(page.url));
  // The frame can report before the page has its title, and the other way round.
  if (!id || !values.player) {
    return { browsing: true };
  }
  return {
    browsing: false,
    id,
    title: values.title,
    channel: values.channel,
    channel_url: values.channel_url,
    ...playback(values.player, page.received_at),
  };
}

function activity(video) {
  if (video.browsing) {
    return { name: NAME, details: t("browsing"), kind: "watching" };
  }

  const buttons = [{ label: t("watch"), url: `https://www.dailymotion.com/video/${video.id}` }];
  if (video.channel_url) buttons.push({ label: t("channel"), url: video.channel_url });

  return {
    name: NAME,
    details: cut(video.title) || t("watching"),
    state: video.channel ? cut(t("by", { channel: video.channel })) : null,
    kind: "watching",
    start: video.start,
    end: video.end,
    // Dailymotion answers this address with a redirect to the current thumbnail.
    large_image: `https://www.dailymotion.com/thumbnail/video/${video.id}`,
    large_text: NAME,
    small_text: video.playing ? t("playing") : t("paused"),
    buttons,
  };
}

// A paused video gives way when something else is playing.
function meta(video) {
  return { paused: !video.browsing && !video.playing };
}
