// Roblox: shows the game being played.
// The player writes what it is doing to a log file; the script follows that log,
// then asks Roblox for the name, creator and icon of the game.

const PLAYER = "RobloxPlayerBeta.exe";
const LOG_SUFFIX = "_last.log";
const LOG_LINES = 500; // How much of the end of the log to read each time.
const RETRY_SECONDS = 60; // Wait before asking again for a game that failed.
const LOG_RECHECK_SECONDS = 60; // How often to look for a newer log file.
const MAX_GAMES = 50; // Game details kept in memory; the oldest go first.

// Lines of the log that tell what the player is doing.
const MENU_LINE = "returnToLuaAppInternal";
const JOINED_LINE = "game_join_loadtime"; // Followed by `universeid:<number>`.
const LEFT_LINE = "shutDown: (stage:UGCGame)";

const ICON = "https://i.pinimg.com/564x/1a/c6/c5/1ac6c5d7cbf6c64b13923d7e258f34a3.jpg";
const LOADING = "https://media1.tenor.com/m/tga0EoNOH-8AAAAC/loading-load.gif";

// What the log says is happening: "menu", "left", or the id of the game
// being played. `since` is when that began, in Unix seconds.
let state = null;
let since = 0;
// Game details by id, kept so each game is requested once.
const games = new Map();
// The log being followed, and when the folder was last searched for it.
let log = null;
let logCheckedAt = 0;

function seconds() {
  return Math.floor(Date.now() / 1000);
}

// The state a log line announces, or null if it says nothing about it.
function stateOf(line) {
  if (line.includes(MENU_LINE)) return "menu";
  if (line.includes(LEFT_LINE)) return "left";
  const joined = line.includes(JOINED_LINE) && /universeid:(\d+)/.exec(line);
  return joined ? joined[1] : null;
}

// The end of the log is read again at every poll, so only its last state
// counts: replaying older lines would restart the clock each time.
function follow(lines) {
  let latest = null;
  for (const line of lines) latest = stateOf(line) ?? latest;
  if (latest !== null && latest !== state) {
    state = latest;
    since = seconds();
  }
}

// The newest log, searched for again only once a minute: the folder can hold
// hundreds of files and the player keeps writing to the same one.
function currentLog() {
  if (!log || seconds() - logCheckedAt >= LOG_RECHECK_SECONDS) {
    log = host.files.latest(host.files.roots[0], LOG_SUFFIX);
    logCheckedAt = seconds();
  }
  return log;
}

// First entry of the `data` list of a Roblox web answer, or null.
function firstOf(url) {
  try {
    const answer = host.http.get(url);
    return answer.status === 200 ? JSON.parse(answer.body).data[0] || null : null;
  } catch {
    return null;
  }
}

// Name, creator, players and icon of a game. A failure is remembered for a
// minute so a Roblox outage does not turn into a request per poll.
function detailsOf(id) {
  const known = games.get(id);
  if (known && (known.info || seconds() - known.failedAt < RETRY_SECONDS)) return known;

  const info = firstOf(`https://games.roblox.com/v1/games?universeIds=${id}`);
  const icon = firstOf(
    `https://thumbnails.roblox.com/v1/games/icons?universeIds=${id}&size=512x512&format=Png&isCircular=false`,
  );
  // A Map keeps insertion order, so its first key is the oldest entry.
  if (!known && games.size >= MAX_GAMES) games.delete(games.keys().next().value);
  const details = { info, icon, failedAt: info ? 0 : seconds() };
  games.set(id, details);
  return details;
}

function read(context) {
  if (!host.process.list().includes(PLAYER)) {
    // A new session writes a new log, so look for it when the player returns.
    state = null;
    log = null;
    return null;
  }

  const file = currentLog();
  if (file) follow(host.files.tail(file, LOG_LINES));

  const window = host.window.foreground();
  const inGame = state !== null && state !== "menu" && state !== "left";
  return {
    state,
    since,
    inGame,
    game: inGame ? detailsOf(state) : null,
    showPlayers: context.options.show_players,
    foreground: Boolean(window && window.process === PLAYER),
  };
}

function menuActivity(roblox) {
  return {
    details: t("menu"),
    state: t("browsing"),
    start: roblox.since,
    large_image: ICON,
    large_text: "Roblox",
    small_image: LOADING,
    small_text: t("loading"),
  };
}

// Page of a game on roblox.com. The id comes from the network, so only a
// positive whole number makes it into the address.
function gamePage(info) {
  const place = info?.rootPlaceId;
  return Number.isSafeInteger(place) && place > 0 ? `https://www.roblox.com/games/${place}` : null;
}

function gameActivity(roblox) {
  const info = roblox.game.info;
  const creator = info && (info.creator?.name || t("unknown"));
  return {
    details: info ? info.name : null,
    details_url: gamePage(info),
    state: info ? t("by", { creator }) : t("playing"),
    start: roblox.since,
    large_image: roblox.game.icon?.imageUrl || ICON,
    large_text: "Roblox",
    small_image: ICON,
    small_text: info && roblox.showPlayers ? t("players", { count: info.playing || 0 }) : null,
  };
}

function activity(roblox) {
  if (roblox.state === null || roblox.state === "left") return null;
  return roblox.inGame ? gameActivity(roblox) : menuActivity(roblox);
}

function meta(roblox) {
  return { foreground: roblox.foreground, application: PLAYER };
}
