import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { commitToBranch, gitEnv, readFromBranch, remoteOf } from "../catalog-branch.mjs";

// A bare repository on disk stands in for GitHub.
function remote(t) {
  const dir = mkdtempSync(join(tmpdir(), "catalog-remote-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  execFileSync("git", ["init", "--quiet", "--bare", dir]);
  const git = (...args) => execFileSync("git", args, { cwd: dir, encoding: "utf8" }).trim();
  return { dir, git };
}

test("replacing leaves the branch with one commit that holds only the file", (t) => {
  const { dir, git } = remote(t);
  const publish = (content) =>
    commitToBranch({
      remote: dir,
      branch: "catalog",
      name: "catalog.json",
      content,
      message: "publish",
      replace: true,
    });
  assert.equal(publish("first"), true);
  assert.equal(publish("second"), true);
  assert.equal(git("rev-list", "--count", "catalog"), "1");
  assert.equal(git("ls-tree", "--name-only", "catalog"), "catalog.json");
  assert.equal(readFromBranch({ remote: dir, branch: "catalog", name: "catalog.json" }), "second");
});

test("adding keeps the history and the other files of the branch", (t) => {
  const { dir, git } = remote(t);
  const add = (name, content) =>
    commitToBranch({ remote: dir, branch: "catalog-stats", name, content, message: `add ${name}`, replace: false });
  assert.equal(add("retired.json", "{}"), true);
  assert.equal(add("stats.json", "one"), true);
  assert.equal(add("stats.json", "two"), true);
  assert.equal(git("rev-list", "--count", "catalog-stats"), "3");
  assert.equal(git("ls-tree", "--name-only", "catalog-stats"), "retired.json\nstats.json");
  assert.equal(readFromBranch({ remote: dir, branch: "catalog-stats", name: "retired.json" }), "{}");
  // The same contents again are not a new commit.
  assert.equal(add("stats.json", "two"), false);
  assert.equal(git("rev-list", "--count", "catalog-stats"), "3");
});

test("a branch or a file that does not exist reads as absent, and a remote that fails does not", (t) => {
  const { dir } = remote(t);
  assert.equal(readFromBranch({ remote: dir, branch: "catalog-stats", name: "retired.json" }), null);
  commitToBranch({
    remote: dir,
    branch: "catalog-stats",
    name: "stats.json",
    content: "x",
    message: "m",
    replace: false,
  });
  assert.equal(readFromBranch({ remote: dir, branch: "catalog-stats", name: "retired.json" }), null);
  // A ledger that cannot be read must stop the run instead of passing for an empty one.
  assert.throws(() => readFromBranch({ remote: join(dir, "missing"), branch: "catalog-stats", name: "retired.json" }));
});

test("names that could change a command are refused before git runs", () => {
  const change = { remote: "nowhere", content: "", message: "m", replace: true };
  for (const branch of ["", "--force", "a b", "refs/heads/x", "A", "x".repeat(64)]) {
    assert.throws(() => commitToBranch({ ...change, branch, name: "a.json" }), /branch/, branch);
  }
  for (const name of ["", "-rf", "../a.json", "a/b.json", ".git", "a b"]) {
    assert.throws(() => commitToBranch({ ...change, branch: "catalog", name }), /file/, name);
    assert.throws(() => readFromBranch({ remote: "nowhere", branch: "catalog", name }), /file/, name);
  }
  assert.equal(remoteOf("owner/name-catalog"), "https://github.com/owner/name-catalog.git");
  for (const repo of ["", "owner", "a/b/c", "owner/na me", "https://evil.example/a/b"]) {
    assert.throws(() => remoteOf(repo), /--repo/, repo);
  }
});

test("the token travels in git's environment, only for github.com", () => {
  const env = gitEnv("secret-token", { PATH: "p" });
  assert.equal(env.GIT_CONFIG_KEY_0, "http.https://github.com/.extraheader");
  assert.equal(
    env.GIT_CONFIG_VALUE_0,
    `AUTHORIZATION: basic ${Buffer.from("x-access-token:secret-token").toString("base64")}`,
  );
  assert.equal(env.GIT_TERMINAL_PROMPT, "0");
  // Without a token git asks for nothing and is given nothing.
  assert.deepEqual(gitEnv("", { PATH: "p" }), { PATH: "p", GIT_TERMINAL_PROMPT: "0" });
});
