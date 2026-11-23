# 0002 -- The MCP server owns MCP

<details>
<summary><strong>Status:</strong> Drafted 2026-11-22</summary>

| Date       | Transition |
| ---------- | ---------- |
| 2026-11-22 | Drafted    |

</details>

## Context

The game page can't accept connections, so the project is two halves: a Rust
binary on the host that the MCP client spawns over stdio, and a Netscript
script inside the game that dials out to it over a websocket. Something has to
own the MCP protocol, and the skeleton put it in the game. The Rust half is a
dumb pipe that relays JSON-RPC lines and never parses them, and `mcp.js`
answers everything.

Three things make that the wrong way round.

**The first thing a newcomer sees is the game not running.** The guide is
for players who can't program, installed as a Claude Desktop extension. Most
of the time they open Desktop before they've opened Bitburner, or before
they've run the script. With the game owning MCP, the client's `initialize`
sits in the relay's backlog until the game connects, so the client hangs or
times out with nothing useful to say. The answer that helps is "open
Bitburner and run `mcp/mcp.js`", and only the host side is there to give it.

**The game side restarts and the client doesn't.** Installing augmentations
kills every script, and so does reloading the game. A fresh `mcp.js` has never
seen the client's `initialize`, so either the relay replays the handshake,
which means parsing MCP, or the session breaks every time the player installs.

**Tiers have to be enforced before anything reaches the game.** The tier
switch (analyst, pair programmer, player agent) is a promise that a tier the
player turned off can't be reached. Filtering `tools/list` and refusing
`tools/call` means reading MCP on the host. Once the host has to parse MCP
anyway, keeping it a pipe buys nothing.

## Decision

The MCP server, the Rust half in `mcp-server/`, owns MCP. It answers the
handshake, serves a fixed tool catalog filtered by tier, and validates calls
against it. The game only executes tools.

- The catalog comes from one definitions file that both halves read at build
  time, so a tool's name and schema are written once.
- The MCP server and the game talk over a small protocol of our own, not MCP.
  It opens with a version check, and a mismatch tells the game side to update
  instead of guessing.
- A tool call made while the game is disconnected returns a tool error
  telling the player to open Bitburner and run `mcp/mcp.js`. It doesn't hang
  and it doesn't queue.
- The transport to the client stays stdio.

## Consequences

What this buys:

- The handshake and `tools/list` work with no game at all, so a client
  connects cleanly, and a newcomer gets told what to do next.
- The client's session outlives the game side. Reloads and augmentation
  installs cost a reconnect on the websocket, not a broken MCP session.
- Tiers are enforced in one place that the game can't route around.
- Protocol handling is ordinary Rust, testable with `cargo test` and no game
  running. That matters because testing in the game means a human pushing
  files and typing `run`.
- Tool descriptions and schemas cost no game RAM. This is small, since strings
  are free in the RAM calculation, but it keeps the resident script about
  executing tools and nothing else.

What it costs:

- Each tool has two halves in two languages: its definition on the host, its
  implementation in the game. The shared definitions file keeps the names and
  schemas honest, but nothing stops the two from disagreeing about behaviour.
- We now own a second protocol and its versioning. The game side can be older
  or newer than the MCP server: a player updates one and not the other, or
  their save holds stale files. The version check is what keeps that from being
  a silent failure.
- The MCP server is no longer a pipe, so it has to keep up with MCP itself:
  protocol versions, capabilities, and whatever the clients we care about
  actually send.
- A call that's in flight when the game drops needs an answer of its own,
  distinct from "not connected".

## Alternatives considered

**The game owns MCP, and the host side is a filtering pipe.** This is what the
skeleton does, minus the filtering. The handshake still hangs until the game
connects, and filtering tiers means parsing MCP on the host anyway, so the
host side ends up understanding the protocol without getting any of the
benefits of owning it.

**Split: the MCP server handles the handshake and passes the game's tool list
through.** The client would connect straight away with an empty tool list,
then get tools once the game connects. This depends on clients honouring
`notifications/tools/list_changed` and re-fetching, and we can't assume that
of every client. A newcomer who sees no tools has no way of knowing they
should start the game.

**A long-lived daemon speaking Streamable HTTP.** This would let several
clients share one game and survive client restarts. It's a transport question,
not an ownership one, and either owner would work over it. The Desktop
extension wants stdio, so stdio stays. If we revisit the transport later, the
MCP server owning MCP makes the switch easier, not harder.

## Open questions

- The format of the definitions file, and how each half reads it at build
  time.
- The shape of the game protocol beyond the version check: frames, ids,
  errors, and what a call in flight gets when the game drops.
- Whether to build on an MCP crate such as `rmcp` or answer the handful of
  methods we need by hand.
