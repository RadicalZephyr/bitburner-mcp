# Handoff: verify the bitburner-mcp skeleton where the tools exist

Date: 2026-09-24
From: a session with cargo 1.98 but no Node and no C linker
To: a container with Node 22, a linker, and ideally the two sibling repos

## Where things stand

The repo is a fresh polyglot skeleton and entirely uncommitted; the
branch has no commits at all. Nothing has ever been built. The design,
the constraints the game imposes, and the open questions are already
written up. Do not re-derive them:

- `README.md`: shape, what the game imposes, build and run, open questions.
- `CLAUDE.md`: commands, pipe semantics, constraints with file pointers
  into `../bitburner-src`.
- `ingame/src/main.ts`, `ingame/vite.config.ts`, `ingame/tsconfig.json`,
  `ingame/package.json`: the in-game half.
- `proxy/src/main.rs`, `proxy/Cargo.toml`: the relay.

The previous session ended by asking her to run the builds herself. She
is moving the work to a container that can instead. That is the whole
job of the next session before anything else happens.

## First job: make it build, fix what breaks

None of this has executed. Run it in order and read the output rather
than trusting exit codes.

1. `cd ingame && npm install`. This creates `package-lock.json`; keep it.
2. `npm run build`. Expect exactly one file, `dist/mcp.js`, with an
   exported `main` (Vite usually ends the file with `export { main };`).
   `rolldownOptions.output.codeSplitting: false` was checked against
   rolldown 1.2.10's typings only; Vite 8.3 may warn or ignore it in lib
   mode. The deprecated spelling is `inlineDynamicImports: true`.
3. `npm run typecheck`. This is `tsc --noEmit` against the copied
   `NetscriptDefinitions.d.ts` via the `@ns` alias. `paths` is used
   without `baseUrl`, and `types` is `[]`, both untested.
4. `grep -nwE 'window|document' ingame/dist/mcp.js` must print nothing.
   See CLAUDE.md, Static RAM.
5. `cd proxy && cargo clippy --all-targets -- -D warnings && cargo build`.
   The crate has never been type-checked: the previous session got only
   as far as dependency build scripts failing on the missing linker. API
   use was verified by reading the tungstenite 0.30 and tokio-tungstenite
   0.30 sources. Most likely trouble spots: the `tokio::select!` in `main`
   (precondition `if stash.is_none()` plus a diverging match arm),
   `Message::Text(line.into())`, and `tracing::debug!(?other, ...)`.
6. `cargo fmt --check` already passes.

Then run the smoke test under README.md, Running. The game dials the
proxy, so if the proxy runs in the container and the game on the host,
publish port 12526 from the container, or run the proxy on the host.

## Decisions the code embodies but does not explain

- TypeScript is pinned to `^5.9.3`, not the current 7.0.2 (the Go port),
  to match `bitburner-src` and her scripts repo. Prettier is pinned to
  3.6.2 for the same reason. Node is `lts/jod` via `.nvmrc`, which mise
  picks up because of `ingame/mise.toml`.
- Port 12526 is one above the Remote File API's 12525, which
  `bitburner-filesync` in `../bitburner-scripts` uses. Nothing else about
  it is deliberate.
- `ingame/NetscriptDefinitions.d.ts` is a byte copy from `bitburner-src`
  3.0.2. Her scripts repo does the same and refreshes it via filesync.
- The proxy buffers while the game is disconnected (one stashed line plus
  a 64-entry channel) so an MCP client's `initialize` waits for the game
  instead of failing. Not discussed with her.
- `ping` exists only so the pipe can be tested; everything else returns
  `-32601`.
- `.dir-locals.el` at the root is for her Emacs and tide setup; her
  global gitignore hides it, so it will not show in `git status`.

## After the builds pass

Nothing further was agreed. Her working rules live in her global
CLAUDE.md; check it is loaded in the new container. The short form:
conversation first, agree interface and tests before non-trivial code,
increments reviewable in five minutes, push back on weak reasoning. The
last message to her proposed agreeing the first tool set and its tests
before any protocol code. The first real forks are in README.md, Open
questions: a resident script with a curated tool set versus a thin
dispatcher plus helper scripts (RAM), and stdio versus a long-lived
daemon (transport). Do not decide these alone.

She did not ask for a commit. Ask before making the first one, and if
she wants it, include `package-lock.json` and `proxy/Cargo.lock`.

## Memory

Two memories were saved in this project's memory directory.
`sandbox-has-no-node-or-linker` describes the old container and is
probably wrong in the new one: re-check and delete or rewrite it.
`zefira-tooling-preferences` should still hold. If the new container
does not share the home directory, neither file exists there.

## Environment facts gathered on 2026-09-24

- Bitburner 3.0.2 in `../bitburner-src`; Electron ^41.4.0, which is
  Chromium 146 and Node 24. The renderer has context isolation and the
  preload exposes save and export IPC only.
- npm latest: vite 8.3.0 (depends on rolldown ~1.2.6), rolldown 1.2.10,
  typescript 7.0.2, prettier 3.9.9, bitburner-filesync 1.2.0-beta4.
- `proxy/Cargo.lock` resolved tokio 1.53.1 and tokio-tungstenite 0.30.0.
- If `../bitburner-src` or `../bitburner-scripts` is missing in the new
  container, the file pointers in CLAUDE.md will not resolve. The game
  source is bitburner-official/bitburner-src on GitHub; use the tag
  matching 3.0.2.

## Suggested skills

- `run` for the end-to-end smoke test once both halves build.
- `engineering:testing-strategy` when agreeing tests for the first tool
  set, before writing any.
- `code-review` (or `simplify`) on the skeleton before the first commit;
  the Rust half has never been seen by a compiler.
- `anthropic-skills:consolidate-memory` to retire the stale sandbox memory.
- `engineering:architecture` only when transport or session state gets
  decided. Her rule is that decisions go in `docs/decisions/` using her
  own template, marked draft, so adapt the ADR to that rather than the
  skill's default format.
