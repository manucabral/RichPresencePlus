// Runs the coverage-guided fuzz targets of fuzz/ for a while each.
//
//   node tools/fuzz.mjs [seconds per target, 60 by default] [target...]
//
// Needs a nightly toolchain and cargo-fuzz:
//   rustup toolchain install nightly --profile minimal
//   cargo install cargo-fuzz --locked
//
// The targets build with AddressSanitizer. On Windows its runtime comes with the C++
// build tools of Visual Studio, and this script puts it on the PATH. What each run
// learns stays in fuzz/corpus (not kept in git); fuzz/seeds holds the starting inputs.
// An input that breaks a target is saved under fuzz/artifacts and the script fails.
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync } from "node:fs";
import { delimiter, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const FUZZ = join(ROOT, "fuzz");
const TARGETS = readdirSync(join(FUZZ, "fuzz_targets"))
  .filter((name) => name.endsWith(".rs"))
  .map((name) => name.slice(0, -3))
  .sort();

/** The folder with the AddressSanitizer runtime of the newest Visual Studio tools. */
function sanitizerFolder() {
  if (process.platform !== "win32") return null;
  for (const base of [process.env["ProgramFiles(x86)"], process.env.ProgramFiles]) {
    const studio = join(base ?? "", "Microsoft Visual Studio");
    if (!existsSync(studio)) continue;
    for (const version of readdirSync(studio).sort().reverse()) {
      for (const edition of readdirSync(join(studio, version))) {
        const tools = join(studio, version, edition, "VC", "Tools", "MSVC");
        if (!existsSync(tools)) continue;
        for (const release of readdirSync(tools).sort().reverse()) {
          const folder = join(tools, release, "bin", "Hostx64", "x64");
          if (existsSync(join(folder, "clang_rt.asan_dynamic-x86_64.dll"))) return folder;
        }
      }
    }
  }
  throw new Error("no AddressSanitizer runtime: install the C++ build tools of Visual Studio");
}

function main() {
  const [first, ...rest] = process.argv.slice(2);
  const seconds = /^\d+$/.test(first ?? "") ? Number(first) : 60;
  const asked = /^\d+$/.test(first ?? "") ? rest : [first, ...rest].filter(Boolean);
  const unknown = asked.filter((name) => !TARGETS.includes(name));
  if (unknown.length > 0) throw new Error(`unknown targets: ${unknown.join(", ")}; there are ${TARGETS.join(", ")}`);
  const env = { ...process.env };
  const runtime = sanitizerFolder();
  if (runtime) env.PATH = `${runtime}${delimiter}${env.PATH}`;

  const broken = [];
  for (const target of asked.length > 0 ? asked : TARGETS) {
    const corpus = join(FUZZ, "corpus", target);
    const seeds = join(FUZZ, "seeds", target);
    mkdirSync(corpus, { recursive: true });
    const folders = existsSync(seeds) ? [corpus, seeds] : [corpus];
    console.log(`== ${target}, ${seconds} s`);
    const run = spawnSync(
      "cargo",
      ["+nightly", "fuzz", "run", target, "-O", ...folders, "--", `-max_total_time=${seconds}`, "-rss_limit_mb=2048"],
      { cwd: ROOT, env, stdio: ["ignore", "inherit", "pipe"], encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
    );
    const lines = (run.stderr ?? "").split(/\r?\n/);
    const done = lines.findLast((line) => line.startsWith("Done "));
    const coverage = lines.findLast((line) => line.includes(" cov: "));
    if (run.status === 0) {
      console.log(`ok   ${done ?? "finished"}${coverage ? `, ${coverage.match(/cov: \d+/)?.[0]}` : ""}`);
    } else {
      broken.push(target);
      console.log(lines.filter((line) => /ERROR|panicked|SUMMARY|Test unit written|artifact/.test(line)).join("\n"));
      console.log(`FAIL ${target}: see fuzz/artifacts/${target}`);
    }
  }
  console.log(broken.length > 0 ? `${broken.length} failed: ${broken.join(", ")}` : "all passed");
  process.exitCode = broken.length > 0 ? 1 : 0;
}

main();
