# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

An MCP server for Bitburner, split in two: `ingame/` (TypeScript, a Netscript script that runs inside the game page and owns all MCP tools) and `proxy/` (Rust, a stdio-to-websocket relay on the host, needed because the page cannot accept connections). `README.md` has the design and the open questions; read it before changing the shape of either half.

## Commands

Each half is built from its own directory. There is no root build.

`ingame/` (Node 22 via `.nvmrc`; mise or nvm):

- `npm install` once, then `npm run build` produces `dist/mcp.js` (Vite 8 on rolldown, one ES module)
- `npm run watch` rebuilds on change
- `npm run typecheck` runs `tsc --noEmit`; Vite does not type-check
- `npm run format` / `npm run format:check` (prettier; config is `.prettierrc` at the repo root)

`proxy/`:

- `cargo build` (`--release` for the binary an MCP client should spawn)
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt` before committing
- `cargo run` listens on `127.0.0.1:12526`; `BITBURNER_MCP_BIND` overrides. Logs go to stderr; stdout carries only JSON-RPC.

No test runner is set up on either side yet. Once there is one, Rust is `cargo test <name>`; the TypeScript choice is still open.

## How the pipe works

MCP client, stdio (newline-delimited JSON-RPC), proxy, websocket (one message per text frame), `mcp.js`.

- The proxy never parses MCP. All protocol logic belongs in `ingame/src/main.ts`, which currently answers `ping` and rejects everything else with `-32601`.
- While the game is disconnected the proxy holds client lines in a small backlog (one stashed line plus a 64-entry channel) and stops reading stdin beyond that. stdin EOF ends the process; the game dropping its socket does not, it just waits for a reconnect.
- The port lives in exactly two places: `DEFAULT_BIND` in `proxy/src/main.rs` and `DEFAULT_URL` in `ingame/src/main.ts`. Change both.

## Constraints from the game

The reference is `../bitburner-src` (3.0.2). Verify there rather than from memory when something looks off.

- **No overlapping Netscript calls.** `checkEnvFlags` in `src/Netscript/NetscriptHelpers.tsx` runs on every `ns` call and kills the script if another async `ns` call is still pending. `asleep` is exempt and never marks itself as running, which is why `main` parks on `ns.asleep` or plain promises while the websocket handlers do the work. Any tool that awaits an async `ns` call must be serialised behind a queue.
- **Static RAM.** `src/Script/RamCalculations.ts` walks the bundle's AST: every referenced `ns.*` function adds its cost (`src/Netscript/RamCostGenerator.ts`), and the identifiers `window` and `document` add 25GB each. Keep them out of the bundle, dependencies included. Check the cost with `mem mcp.js` in the game or the Remote File API's `calculateRam`.
- **One file.** The game loads a script as a single blob-URL ES module on the main thread (`src/NetscriptJSEvaluator.ts`). The build must emit exactly `dist/mcp.js`: no chunks, no dynamic imports. The entry point contract is `export async function main(ns)`.
- **Kill semantics.** Killing the script runs the `ns.atExit` handler (which closes the socket) and makes the next `ns` call throw `ScriptDeath` (`src/Netscript/ScriptDeath.ts`). A catch-all around `ns` calls in a tool handler must rethrow it, or the script turns into a zombie that keeps answering requests.
- **Types.** `@ns` resolves to `ingame/NetscriptDefinitions.d.ts`, a copy of `../bitburner-src/src/ScriptEditor/NetscriptDefinitions.d.ts`. Refresh it by copying again when the game updates (or via the RFA `getDefinitionFile` method). It is excluded from prettier.
- **Target.** Electron 41 is Chromium 146, so `build.target` is `esnext` and nothing is downlevelled. Vite 8 bundles with rolldown and transforms with oxc: the config keys are `build.rolldownOptions` and `oxc`, not `rollupOptions` and `esbuild`.

## Neighbouring repos

- `../bitburner-src`: game source. Entry points worth knowing: `src/RemoteFileAPI/` (the outbound-websocket precedent and the file-push protocol on port 12525) and `electron/` (window preferences and the preload, which is why the page has no Node access).
- `../bitburner-scripts`: the user's own game scripts. Its `bitburner-filesync` config pushes `dist/` into the game over port 12525; pointing it at `ingame/dist` is the quickest way to load `mcp.js`.
