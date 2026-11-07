// Creator Tools: shows which tool is in front. It looks only at the name of the
// program and never at the window title, which holds file and project names.

const TOOLS = new Map([
  ["code.exe", ["Visual Studio Code", "coding"]],
  ["cursor.exe", ["Cursor", "coding"]],
  ["devenv.exe", ["Visual Studio", "coding"]],
  ["idea64.exe", ["IntelliJ IDEA", "coding"]],
  ["pycharm64.exe", ["PyCharm", "coding"]],
  ["sublime_text.exe", ["Sublime Text", "coding"]],
  ["figma.exe", ["Figma", "designing"]],
  ["photoshop.exe", ["Photoshop", "designing"]],
  ["illustrator.exe", ["Illustrator", "designing"]],
  ["afterfx.exe", ["After Effects", "editing"]],
  ["adobe premiere pro.exe", ["Premiere Pro", "editing"]],
  ["resolve.exe", ["DaVinci Resolve", "editing"]],
  ["reaper.exe", ["REAPER", "music"]],
  ["fl64.exe", ["FL Studio", "music"]],
  ["blender.exe", ["Blender", "modeling"]],
]);

// Looking something up for a minute is still working in the tool.
const GRACE_SECONDS = 120;

let current = null;
let since = 0;
let seen = 0;

function read(context) {
  const window = host.window.foreground();
  const program = window ? window.process.toLowerCase() : "";
  if (TOOLS.has(program)) {
    if (program !== current) {
      current = program;
      since = context.now;
    }
    seen = context.now;
  } else if (current && context.now - seen > GRACE_SECONDS) {
    current = null;
  }
  if (!current) return null;
  const [name, work] = TOOLS.get(current);
  return { name, work, since, application: current };
}

function activity(tool) {
  return { name: tool.name, details: t(tool.work), kind: "playing", start: tool.since };
}

// Lets the user exclude a tool by its program name.
function meta(tool) {
  return { application: tool.application };
}
