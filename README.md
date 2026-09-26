# bitburner-mcp

An MCP server that lives inside a running Bitburner game, so a model can
read and act on live game state through the Netscript API instead of
guessing from screenshots or save files.

Status: skeleton, 2026-09-24. The pipe works end to end for `ping` and
nothing else.

## Shape

Two pieces, two languages.

```
MCP client --stdio--> proxy (Rust, on the host) <--websocket-- mcp.js (TypeScript, inside Bitburner)
```

- `ingame/` builds `dist/mcp.js`, a single-file Netscript script. It runs
  inside the game page, owns every MCP tool, and dials out to the proxy.
- `proxy/` builds `bitburner-mcp-proxy`. An MCP client spawns it and talks
  MCP over stdio; it accepts the game's websocket and relays JSON-RPC
  lines both ways. It never interprets MCP.

The split exists because the game page cannot listen for connections.
Bitburner's Electron 41 renderer runs with context isolation and a
preload that exposes only save/export IPC, so an outbound websocket is
the only channel. This is the same trick the game's own Remote File API
uses.

The in-game half is a Netscript script rather than something injected
into the page because `ns` is the supported, structured view of game
state. Reaching into the React internals is the alternative, and it
breaks on every release.

## What the game imposes

Read from `bitburner-src` 3.0.2; file paths are in `CLAUDE.md`.

- RAM is the budget. A script pays 1.6GB to exist plus a static cost for
  every `ns` function it references. The identifiers `window` and
  `document` cost 25GB each. So the tool surface is the RAM footprint,
  and a broad server probably has to dispatch work to short-lived helper
  scripts via `ns.run` rather than reference everything itself.
- Netscript calls must not overlap. While one async `ns` call is pending,
  any other `ns` call kills the script. `ns.asleep` is the one exception,
  which is why the main loop parks on it and websocket handlers do the
  work. Tool calls will need a queue.
- A script is one ES module loaded from a blob URL on the main thread,
  exporting `main(ns)`. `WebSocket` is available; the bundle must be one
  file with no dynamic imports.
- Electron 41 is Chromium 146, so the bundle targets `esnext` and nothing
  is transpiled.

## Building

`ingame/` needs Node 22, pinned in `ingame/mise.toml`:

```bash
cd ingame && npm install && npm run build
```

Output is `ingame/dist/mcp.js`. `npm run typecheck` runs tsc against the
copied `NetscriptDefinitions.d.ts`.

`proxy/` is plain cargo:

```bash
cd proxy && cargo build --release
```

## Running

1. Register the proxy with your MCP client as a stdio server, e.g.
   `claude mcp add bitburner -- /path/to/bitburner-mcp-proxy`. It listens
   on `127.0.0.1:12526` (`BITBURNER_MCP_BIND` overrides) and logs to
   stderr.
2. Get `mcp.js` into the game: `bitburner-filesync` pointed at
   `ingame/dist`, or paste it into the in-game editor.
3. `run mcp.js` in the game terminal. It connects, and reconnects every
   5s if the proxy goes away. Kill the script to stop.
4. Smoke test without a client: run the proxy in a terminal and type
   `{"jsonrpc":"2.0","id":1,"method":"ping"}`. The game answers
   `{"jsonrpc":"2.0","id":1,"result":{}}`.

## Open questions

Not decided; nothing costly depends on them yet.

- Transport. stdio means one client, and the proxy dies with it. A
  long-lived daemon speaking Streamable HTTP would let several clients
  share one game and survive client restarts, at the cost of a real
  server in the proxy.
- Session state. If the game reconnects mid-session, who replays MCP
  `initialize`? The proxy could own the handshake itself, which stops it
  being a dumb pipe.
- Tool surface versus RAM. One resident script with a curated tool set,
  or a thin resident dispatcher plus per-tool helper scripts.
- Should the proxy also speak the Remote File API so it can push
  `mcp.js` into the game itself? The game holds one RFA connection at a
  time, so this would fight with `bitburner-filesync`.
