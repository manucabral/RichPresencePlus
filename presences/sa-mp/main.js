// SA-MP: shows the San Andreas Multiplayer server being played.
//
// The launcher starts gta_sa.exe with `-h <address> -p <port>`. The manifest
// asks for those two options only, so the nickname (`-n`) and a server
// password (`-z`) never reach this script. The server's name, mode and
// players come from the public SA-MP query protocol, which also works with
// open.mp servers. Nothing reads the game's memory.

const PROCESS = "gta_sa.exe";
const DEFAULT_PORT = 7777;
const IMAGE = "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/12120/header.jpg";
// Discord shows a group size only up to this many.
const MAX_PARTY = 1000;

// Server being played (`address:port`) and the Unix second it started.
let currentTarget = null;
let since = 0;

function seconds() {
  return Math.floor(Date.now() / 1000);
}

// The server the game was launched for, or null when it was started without one.
function launchTarget(args) {
  if (!args || typeof args["-h"] !== "string" || args["-h"].trim() === "") return null;
  const port = Number.parseInt(args["-p"], 10);
  return {
    address: args["-h"].trim(),
    port: Number.isInteger(port) && port > 0 && port <= 65535 ? port : DEFAULT_PORT,
  };
}

// Server information, or null while it does not answer. The host refuses a
// question asked too soon; the last answer is kept for that case.
let lastInfo = null;
function serverInfo(target) {
  try {
    lastInfo = host.query("samp", target.address, target.port);
  } catch {
    // Too soon or unresolvable: keep what was known.
  }
  return lastInfo;
}

function read(context) {
  const target = launchTarget(host.process.args(PROCESS));
  if (!target) {
    currentTarget = null;
    lastInfo = null;
    return null;
  }
  const key = `${target.address}:${target.port}`;
  if (key !== currentTarget) {
    currentTarget = key;
    since = seconds();
    lastInfo = null;
  }
  const window = host.window.foreground();
  return {
    target: key,
    info: serverInfo(target),
    since,
    showPlayers: context.options.show_players,
    showAddress: context.options.show_address,
    foreground: Boolean(window?.process && window.process.toLowerCase() === PROCESS),
  };
}

// `[players, slots]` when Discord can show it as a group size.
function party(info) {
  const ok =
    info &&
    Number.isInteger(info.players) &&
    Number.isInteger(info.max_players) &&
    info.players >= 1 &&
    info.players <= info.max_players &&
    info.max_players <= MAX_PARTY;
  return ok ? [info.players, info.max_players] : null;
}

function activity(samp) {
  const { info, target, since, showPlayers, showAddress } = samp;
  const size = showPlayers ? party(info) : null;
  // Counts Discord cannot show as a group go in the text instead.
  const counted =
    showPlayers && !size && info && Number.isInteger(info.max_players) && info.max_players > 0
      ? t("players", { count: info.players, max: info.max_players })
      : null;
  const mode = info?.gamemode || null;
  const smallText = showAddress ? target : info?.language || null;
  return {
    details: info?.hostname || t("playing"),
    state: [mode, counted].filter(Boolean).join(" - ") || null,
    start: since,
    party_size: size,
    large_image: IMAGE,
    large_text: "San Andreas Multiplayer",
    small_image: smallText ? IMAGE : null,
    small_text: smallText,
  };
}

function meta(samp) {
  return {
    foreground: samp.foreground,
    application: PROCESS,
  };
}
