// Desktop Media: shows what plays in a program, through the session Windows
// keeps for media keys. Browsers report there too; the extension covers them
// with more detail, so they are left out.

// Part of the name each browser reports. The last one is Firefox's identifier.
const BROWSERS = [
  "chrome",
  "msedge",
  "edge",
  "firefox",
  "brave",
  "opera",
  "vivaldi",
  "yandex",
  "browser",
  "308046b0af4a39cb",
];

function programOf(app) {
  const name = app.split("!")[0].split("\\").pop();
  return name
    .replace(/\.exe$/i, "")
    .replace(/_[a-z0-9]{13}$/i, "")
    .split(".")
    .pop();
}

function read(context) {
  const media = host.media.current();
  if (!media?.title) return null;

  const app = (media.app || "").toLowerCase();
  if (BROWSERS.some((browser) => app.includes(browser))) return null;

  const playing = media.status === "playing";
  return {
    title: media.title,
    artist: media.artist,
    album: media.album,
    program: programOf(media.app || ""),
    app: media.app || "",
    playing,
    start: playing ? Math.round(context.now - media.position) : null,
    end: playing && media.duration > 0 ? Math.round(context.now - media.position + media.duration) : null,
  };
}

function activity(media) {
  return {
    details: cut(media.title) || t("playing"),
    state: cut([media.artist, media.album].filter(Boolean).join(" - ")) || null,
    // An artist means music; anything else is a video.
    kind: media.artist ? "listening" : "watching",
    start: media.start,
    end: media.end,
    large_text: cut(media.program),
  };
}

function meta(media) {
  return { paused: !media.playing, application: media.app };
}
