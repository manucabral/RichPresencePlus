// Nepu: shows the movie or episode on the page.
// The browser extension reads the page; what it reads is named in manifest.json.

const MAX_IMAGE = 512; // Discord refuses a longer image address.

// The cover is a CSS background: `background-image: url("...")`.
function coverOf(style) {
  const found = /url\(["']?([^"')]+)["']?\)/.exec(style || "");
  const address = found ? found[1] : null;
  return address && /^https?:\/\//.test(address) && address.length <= MAX_IMAGE ? address : null;
}

// "movie" for a movie page, "episode" for a season, episode or player page.
function kindOf(path) {
  if (path.includes("/movie/")) return "movie";
  if (/\/(season|episode|watch)/.test(path)) return "episode";
  return null;
}

function read(context) {
  if (context.pages.length === 0) return null;

  // Prefer a tab that has a title on it over one that is only browsing.
  const watched = context.pages.find((p) => kindOf(parseUrl(p.url).path) && p.values.title);
  if (!watched) return { kind: null };

  const values = watched.values;
  return {
    kind: kindOf(parseUrl(watched.url).path),
    title: values.title,
    heading: values.heading,
    episode: values.episode || values.episode_nav,
    cover: coverOf(values.cover),
    address: watched.url,
  };
}

function activity(page) {
  if (page.kind === null) {
    return { details: t("browsing"), kind: "watching", large_image: "logo", large_text: "Nepu" };
  }

  // A movie shows its other name; an episode shows its number or name.
  const state = page.kind === "movie" ? page.heading : page.episode || page.heading;
  return {
    details: cut(page.title),
    state: cut(state),
    kind: "watching",
    large_image: page.cover || "logo",
    large_text: cut(page.heading) || t("watching"),
    small_image: page.cover ? "logo" : null,
    small_text: page.cover ? t("watching") : null,
    buttons: [{ label: t("watch"), url: page.address }],
  };
}
