# Contributing

Thanks for helping. Please open an issue before a large change, so nobody works
on something that cannot be merged.

## What you need

Rust stable, Node 24 and pnpm 10. On Windows, the WebView2 runtime.

```sh
pnpm --dir frontend install
pnpm --dir frontend build
cargo build -p rpp-desktop --features custom-protocol
```

Without `custom-protocol` the window looks for a development server and shows an
error page. [docs/DEVELOPING.md](docs/DEVELOPING.md) has the rest: tools, values worth
knowing, what to touch to add something, and the decisions behind the design.

## Before you open a pull request

Everything here must pass with no warnings:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny --locked check
pnpm install
pnpm lint
pnpm --dir frontend exec tsc -b
pnpm --dir frontend test
pnpm --dir frontend build
node tools/try-interface.mjs
node tools/check-locales.mjs
node --test "tools/test/*.test.mjs"
pnpm --dir extension test
pnpm --dir website test
```

Changes to the extension's rules also have `node tools/try-extension.mjs`, which
runs them in Chrome. Add `--show` to watch it.

Changes to the window, the interface or anything that runs all day also have
`node tools/try-memory.mjs`. It starts the release build, so close the
application first. It watches memory with the window open and in the tray for
about 25 minutes, and fails if moving between screens leaves objects behind.

## How the code is written

- Logic with no input or output goes in `rpp-core`, and time comes in as an
  argument.
- `unsafe` lives only in `rpp-host`, in the modules that call Windows, with a
  `SAFETY:` comment on each block.
- Every public item has a short docstring in Google style: one direct sentence.
- `pnpm lint` checks presences, the extension and the tools with Biome;
  `pnpm lint:fix` formats them and applies the safe fixes.
- Each new behavior has a test, and each fixed bug has a test that failed before
  the fix.
- A presence script receives what its manifest declares and nothing else.
- Texts are short and plain. Comments explain a decision, not the code.

## Commits

Commits follow [Conventional Commits](https://www.conventionalcommits.org/):
`type(scope): summary`, in English, in the imperative and in lower case. The
scope is the part of the project the commit touches, such as `core`, `app`,
`extension`, `interface`, `presences` or `website`.

| Type | When | Example |
| --- | --- | --- |
| `feat` | A new behavior | `feat(presences): add Dailymotion` |
| `fix` | A bug fix | `fix(extension): stop asking again for a site already allowed` |
| `docs` | Documentation only | `docs: explain options in the developer guide` |
| `refactor` | Code that changes shape, not behavior | `refactor(app): split the controller into modules` |
| `test` | Tests only | `test(catalog): cover a bundle with a missing file` |
| `perf` | Less memory or processor time | `perf(service): wake runners only when pages change` |
| `ci` | Workflows | `ci: lint presences on pull requests` |
| `chore` | Anything else: dependencies, tooling | `chore: update Biome` |

A change that breaks presences or settings adds `!` after the scope, as in
`feat(plugins)!: rename settings to options`, and explains the change in the
body.

## Presences

`docs/developers` shows how to write one and explains every field. A presence
is reviewed for what it reads: it asks for the least it needs.

## Security

Please do not report a vulnerability in a public issue. `SECURITY.md` says how to
report one in private.
