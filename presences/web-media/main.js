// Web Media: shows what plays on a site that has no presence of its own.
// It reads the Media Session API and the page's player, which most video and
// music sites fill in, so it needs no selectors. The extension runs it only on
// sites the user allowed one by one, and never on a site with its own presence.

const MAX_IMAGE = 512; // Discord refuses a longer image address.

function read(context) {
  // A page counts when it publishes a session or has a player running.
  const playing = context.pages.filter((p) => p.values.session || (p.values.player && !p.values.player.paused));
  const page = bestPage(playing);
  if (!page) return null;

  const song = page.values.session || {};
  const clock = playback(page.values.player, page.received_at);
  const address = parseUrl(page.url);
  const host = address ? address.host.replace(/^www\./, "") : null;
  return {
    title: song.title || page.title,
    artist: song.artist,
    artwork: song.artwork && song.artwork.length <= MAX_IMAGE ? song.artwork : null,
    site: page.values.site || host,
    host,
    playing: isPlaying(song.state, page.values.player, clock),
    start: clock.start,
    end: clock.end,
  };
}

// Most sites never set `playbackState`, which then reads "none" and says
// nothing about the page. The player answers instead, and a session with no
// player counts as playing.
function isPlaying(state, player, clock) {
  if (state === "playing" || state === "paused") return state === "playing";
  return player ? clock.playing : true;
}

function activity(media) {
  return {
    details: cut(media.title) || t("playing"),
    state: cut(media.artist || media.site),
    // An artist means music; anything else is a video.
    kind: media.artist ? "listening" : "watching",
    start: media.playing ? media.start : null,
    end: media.playing ? media.end : null,
    large_image: media.artwork,
    large_text: cut(media.site),
  };
}

// The host goes to the policy, so an excluded site stays excluded here too.
function meta(media) {
  return { paused: !media.playing, domains: media.host ? [media.host] : [] };
}
