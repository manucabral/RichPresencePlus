// Reads and commits one file on a branch of a GitHub repository, for the
// catalog's index and its install counts.
//
//   node tools/catalog-branch.mjs read    --repo <owner/name> --branch <b> --name <file> --out <path>
//                                         [--else-repo <owner/name>] [--default <text>]
//   node tools/catalog-branch.mjs replace --repo <owner/name> --branch <b> --file <path> --message <m>
//   node tools/catalog-branch.mjs add     --repo <owner/name> --branch <b> --file <path> --message <m>
//
// `replace` leaves the branch with a single commit that holds only the file:
// the index is published that way, so old catalogs do not pile up. `add`
// commits on top of the branch and keeps its other files and its history, as
// the counts need. `read` writes `--default` when the branch or the file does
// not exist yet; any other failure stops it, because an empty stand-in for a
// ledger that could not be read would then be committed over the real one.
// `--else-repo` names where the file was before, read when the first
// repository does not have it yet.
//
// The token comes from GITHUB_TOKEN and reaches git through its environment,
// never through a command line.
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { pathToFileURL } from "node:url";

const BRANCH = /^[a-z][a-z0-9-]{0,62}$/;
const NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$/;
const REPO = /^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/;
const AUTHOR = ["-c", "user.name=github-actions", "-c", "user.email=actions@users.noreply.github.com"];

/** The address git uses for a repository on GitHub. */
export function remoteOf(repo) {
  if (!REPO.test(repo ?? "")) throw new Error("give --repo <owner/name>");
  return `https://github.com/${repo}.git`;
}

/** The environment that hands a token to git for github.com, or the plain one without a token. */
export function gitEnv(token, base = process.env) {
  const env = { ...base, GIT_TERMINAL_PROMPT: "0" };
  if (!token) return env;
  const basic = Buffer.from(`x-access-token:${token}`).toString("base64");
  return {
    ...env,
    GIT_CONFIG_COUNT: "1",
    GIT_CONFIG_KEY_0: "http.https://github.com/.extraheader",
    GIT_CONFIG_VALUE_0: `AUTHORIZATION: basic ${basic}`,
  };
}

function checked(branch, name) {
  if (!BRANCH.test(branch ?? "")) throw new Error("the branch name is not plain");
  if (!NAME.test(name ?? "")) throw new Error("the file name is not plain");
}

/** Runs `work` with git bound to a scratch repository, which is deleted afterwards. */
function inScratch(token, work) {
  const dir = mkdtempSync(join(tmpdir(), "catalog-branch-"));
  const env = gitEnv(token);
  const git = (...args) =>
    execFileSync("git", args, { cwd: dir, env, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  try {
    git("init", "--quiet");
    // The file is committed byte for byte, on any system.
    git("config", "core.autocrlf", "false");
    return work(git, dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** Fetches the tip of a branch into FETCH_HEAD; false when the remote has no such branch. */
function fetched(git, remote, branch) {
  if (git("ls-remote", "--heads", remote, `refs/heads/${branch}`).trim() === "") return false;
  git("fetch", "--quiet", "--depth", "1", remote, `refs/heads/${branch}`);
  return true;
}

/**
 * The text of a file on a branch.
 *
 * @returns {string|null} Null when the branch or the file does not exist.
 * @throws When the remote cannot be reached, so a failure never reads as "absent".
 */
export function readFromBranch({ remote, branch, name, token }) {
  checked(branch, name);
  return inScratch(token, (git) => {
    if (!fetched(git, remote, branch)) return null;
    const listed = git("ls-tree", "--name-only", "FETCH_HEAD", "--", name).trim();
    return listed === name ? git("show", `FETCH_HEAD:${name}`) : null;
  });
}

/**
 * Commits one file to a branch and pushes it.
 *
 * @param {object} change
 * @param {boolean} change.replace True for a branch left with this single commit and file.
 * @returns {boolean} False when the branch already held the same file, so nothing was pushed.
 * @throws When the push is refused, as when the branch moved meanwhile.
 */
export function commitToBranch({ remote, branch, name, content, message, replace, token }) {
  checked(branch, name);
  return inScratch(token, (git, dir) => {
    if (!replace && fetched(git, remote, branch)) git("checkout", "--quiet", "-b", "work", "FETCH_HEAD");
    else git("checkout", "--quiet", "--orphan", "work");
    writeFileSync(join(dir, name), content);
    git("add", "--", name);
    if (git("status", "--porcelain").trim() === "") return false;
    git(...AUTHOR, "commit", "--quiet", "-m", message);
    git("push", "--quiet", ...(replace ? ["--force"] : []), remote, `HEAD:refs/heads/${branch}`);
    return true;
  });
}

function option(args, name, fallback) {
  const at = args.indexOf(`--${name}`);
  return at >= 0 ? args[at + 1] : fallback;
}

function main(argv) {
  const [command, ...args] = argv;
  const target = {
    remote: remoteOf(option(args, "repo")),
    branch: option(args, "branch"),
    token: process.env.GITHUB_TOKEN,
  };

  if (command === "read") {
    const name = option(args, "name");
    const before = option(args, "else-repo");
    const text =
      readFromBranch({ ...target, name }) ??
      (before ? readFromBranch({ ...target, remote: remoteOf(before), name }) : null) ??
      option(args, "default");
    if (text === undefined) throw new Error(`${name} is not on ${target.branch}, and there is no --default`);
    writeFileSync(option(args, "out", name), text);
    return;
  }

  if (command === "replace" || command === "add") {
    const file = option(args, "file");
    const message = option(args, "message");
    if (!file || !message) throw new Error("give --file <path> and --message <text>");
    const pushed = commitToBranch({
      ...target,
      name: basename(file),
      content: readFileSync(file),
      message,
      replace: command === "replace",
    });
    console.log(
      pushed ? `pushed ${basename(file)} to ${target.branch}` : `${target.branch} already has this ${basename(file)}`,
    );
    return;
  }

  throw new Error("commands: read, replace, add");
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    // What git wrote names the remote but never the token, which is not in any command.
    console.error(String(error.stderr || error.message).trim());
    process.exit(1);
  }
}
