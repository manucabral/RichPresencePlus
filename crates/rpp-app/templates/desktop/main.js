// __NAME__: a desktop presence.
// Replace PROGRAM with the program to look for.

const PROGRAM = "example.exe";

function read(context) {
  // `host.process` exists because manifest.json asks for the "process" permission.
  const running = host.process.list().some((name) => name.toLowerCase() === PROGRAM);
  if (!running) return null;

  return { state: context.options.show_state ? t("editing") : null };
}

function activity(program) {
  return { name: "__NAME__", details: t("using"), state: program.state, kind: "playing" };
}
