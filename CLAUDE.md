# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

An MCP server for Bitburner, split in two: `mcp-server/` (Rust, on the host, spawned by the MCP client over stdio) and `ingame/` (TypeScript, a Netscript script that runs inside the game page and dials out to the MCP server over a websocket, because the page cannot accept connections). `README.md` has the shape; `docs/notes/2026-09-24-what-were-building.md` has the design and what's decided. Read both before changing the shape of either half.

Naming: the Rust half is "the MCP server". "Proxy" means the planned in-game stand-in for `NS` that records RAM use, not the Rust half.

## Commands

Each half is built from its own directory. There is no root build.

`ingame/` (Node 22, pinned in `ingame/mise.toml`):

- `npm install` once, then `npm run build` produces `dist/mcp.js` (Vite 8 on rolldown, one ES module)
- `npm run watch` rebuilds on change
- `npm run typecheck` runs `tsc --noEmit`; Vite does not type-check
- `npm run format` / `npm run format:check` (prettier; config is `.prettierrc` at the repo root)

`mcp-server/`:

- `cargo build` (`--release` for the binary an MCP client should spawn)
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt` before committing
- `cargo run` listens on `127.0.0.1:12526`; `BITBURNER_MCP_BIND` overrides. Logs go to stderr; stdout carries only JSON-RPC.

No test runner is set up on either side yet. Once there is one, Rust is `cargo test <name>`; the TypeScript choice is still open.

## How the pipe works

MCP client, stdio (newline-delimited JSON-RPC), MCP server, websocket (one message per text frame), `mcp.js`.

- Today the MCP server is still a relay that never parses MCP, and `ingame/src/main.ts` answers `ping` and rejects everything else with `-32601`. MCP is moving into `mcp-server/`; the game will only execute tools.
- While the game is disconnected the MCP server holds client lines in a small backlog (one stashed line plus a 64-entry channel) and stops reading stdin beyond that. stdin EOF ends the process; the game dropping its socket does not, it just waits for a reconnect.
- The port lives in exactly two places: `DEFAULT_BIND` in `mcp-server/src/main.rs` and `DEFAULT_URL` in `ingame/src/main.ts`. Change both.

## Constraints from the game

The reference is `../bitburner-src` checked out at the `v3.0.1` tag, the version the game runs. Its `dev` branch pre-bumps `package.json` to the next version, so the number there is not a release. Verify there rather than from memory when something looks off.

- **No overlapping Netscript calls.** `checkEnvFlags` in `src/Netscript/NetscriptHelpers.tsx` runs on every `ns` call and kills the script if another async `ns` call is still pending. `asleep` is exempt and never marks itself as running, which is why `main` parks on `ns.asleep` or plain promises while the websocket handlers do the work. Any tool that awaits an async `ns` call must be serialised behind a queue.
- **Static RAM.** `src/Script/RamCalculations.ts` walks the bundle's AST: every referenced `ns.*` function adds its cost (`src/Netscript/RamCostGenerator.ts`), and the identifiers `window` and `document` add 25GB each. Keep them out of the bundle, dependencies included. Check the cost with `mem mcp.js` in the game or the Remote File API's `calculateRam`.
- **One file.** The game loads a script as a single blob-URL ES module on the main thread (`src/NetscriptJSEvaluator.ts`). The build must emit exactly `dist/mcp.js`: no chunks, no dynamic imports. The entry point contract is `export async function main(ns)`.
- **Kill semantics.** Killing the script runs the `ns.atExit` handler (which closes the socket) and makes the next `ns` call throw `ScriptDeath` (`src/Netscript/ScriptDeath.ts`). A catch-all around `ns` calls in a tool handler must rethrow it, or the script turns into a zombie that keeps answering requests.
- **Types.** `@ns` resolves to `ingame/NetscriptDefinitions.d.ts`, the definition file the running game serves through the RFA `getDefinitionFile` method. Refresh it from the game when the game updates. It should be byte-identical to `src/ScriptEditor/NetscriptDefinitions.d.ts` at the matching tag; never copy it from `dev`, which is ahead of the release. It is excluded from prettier.
- **Target.** Electron 41 is Chromium 146, so `build.target` is `esnext` and nothing is downlevelled. Vite 8 bundles with rolldown and transforms with oxc: the config keys are `build.rolldownOptions` and `oxc`, not `rollupOptions` and `esbuild`.

## Neighbouring repos

- `../bitburner-src`: game source. Entry points worth knowing: `src/RemoteFileAPI/` (the outbound-websocket precedent and the file-push protocol on port 12525) and `electron/` (window preferences and the preload, which is why the page has no Node access).
- `../bitburner-scripts`: the user's own game scripts. Its `bitburner-filesync` config pushes `dist/` into the game over port 12525; pointing it at `ingame/dist` is the quickest way to load `mcp.js`.
