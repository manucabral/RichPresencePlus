// Ready Or Not: shows that the game is running.
// It only looks at the list of running programs; it never reads Steam or the
// game's files.

const GAME = "Ready Or Not";
const IMAGE = "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1144200/header.jpg";
// The game process: the shipping build, and the launcher stub that starts it.
const PROCESSES = ["readyornot-win64-shipping.exe", "readyornot.exe"];

// Unix seconds at which the game was first seen running; 0 while it is closed.
let since = 0;

function read() {
  const running = host.process
    .list()
    .map((name) => name.toLowerCase())
    .find((name) => PROCESSES.includes(name));

  if (!running) {
    since = 0;
    return null;
  }
  if (since === 0) since = Math.floor(Date.now() / 1000);
  return { since, application: running };
}

function activity(game) {
  return {
    details: GAME,
    state: t("playing"),
    start: game.since,
    large_image: IMAGE,
    large_text: GAME,
  };
}

// Lets the user exclude the game by its program name.
function meta(game) {
  return { application: game.application };
}
