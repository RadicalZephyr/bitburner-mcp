# What we're building

Date: 2026-09-24

I sent the last session off to build a skeleton before I knew what it
was for. This note is where we landed after stepping back and grilling
the question properly. Nothing here is a recorded decision yet; the
costly ones go in `docs/decisions/` when code is about to depend on them.

## Three shapes, built in order

There are three things this could be:

1. An analyst: reads game state and advises.
2. A pair programmer: helps write, push and debug scripts.
3. A player agent: calls `ns` and plays.

I want all three eventually, in this repo, with the user choosing a tier
in the proxy's configuration so the higher capabilities are closed off
before anything reaches the game. The analyst comes first because the
other two need everything it has.

The audience for the analyst is someone who likes the game's aesthetic
and story but can't program. They're the reason this is worth doing, and
their constraints are the hard ones, so designing for them first means
the version for me is an extension rather than a rewrite. I test on a
fresh save, not my late-game one.

## v1

A read-only guide: it explains what the player is looking at and
suggests what to do next.

- **Client.** Claude Desktop, installed as a one-click extension with
  prebuilt proxy binaries for Windows, macOS and Linux. The proxy stays
  stdio, so other clients work, they just don't get the easy install.
  The tier switch is a setting in the extension.
- **The proxy owns MCP.** It answers the handshake and serves a fixed
  tool catalog, filtered by tier, from a definitions file both halves
  read at build time. The game only executes tools, over a small
  protocol of our own with a version check on connect. When the game
  isn't connected, calls return "open Bitburner and run `mcp/mcp.js`"
  instead of hanging. That's the main reason: a newcomer's first
  experience is almost always "the game isn't running yet".
- **Game knowledge.** The proxy ships the game's own docs
  (`src/Documentation/doc/en/`, pinned to the game version) behind a
  `read_guide` tool. It's a tool rather than only a resource because in
  Desktop the user attaches resources, not the model.
- **Spoilers.** Filtered by default. Docs are tagged by the progress they
  assume, the tool withholds what's past the player's progress
  (`getResetInfo` for BitNode and Source-Files, `getPlayer` for
  factions) and says so, and the player can ask anyway. The people this
  is for came for the story.
- **Install.** The proxy serves the in-game files over HTTP on its
  existing port, built into its binary. The player pastes one line:

    ```
    wget http://localhost:12526/install.js mcp/install.js; run mcp/install.js
    ```

    Only the enabled tiers' helpers are served, so a tier that's off has
    no files in the save at all. Version mismatches make the resident
    re-fetch.

- **In-game shape.** Everything under `mcp/` so it can't collide with
  anyone's own scripts. A thin resident script holds the websocket and
  starts short-lived helper scripts, one per tool. Helpers run on
  whichever rooted server has the most free RAM, falling back to home.
  I have the dispatcher planned already; that's the next conversation.
- **Nothing gets rooted for the player.** The seven zero-port servers
  (n00dles, foodnstuff, sigma-cosmetics, joesguns, nectar-net,
  hong-fang-tea, harakiri-sushi) are 100GB against home's 8, and NUKE
  needs only ports, not hacking level. But the analyst never writes to
  the game, and that promise is what makes the tiers trustworthy. When a
  helper can't fit, the guide tells the player how to NUKE something.
- **Tools.** player, income, network and scripts, plus an `overview` the
  proxy composes by running the others one at a time. Network returns a
  compact table with a filter. Logs and hacknet come next.
- **Build.** Plain `tsc` on the native TypeScript 7; no Vite. No runtime
  dependencies in-game. `importFromGlobal` from my scripts repo is the
  escape hatch if we ever absolutely need one.
- **Platforms.** Both the Steam build and the web build.

## What limits the guide

Checked against `bitburner-src` 3.0.2:

- A new save has 8GB of home RAM and every script pays 1.6GB to exist.
  A full snapshot in one script would be about 7GB, which is why the
  helpers are small and run one at a time.
- Without Source-File 4 the Singularity API throws, and the script is
  still charged 16x RAM just for mentioning a function. So v1 can't see
  faction or company reputation, invitations, which augmentations a
  faction sells, queued augmentations or current work. It gets joined
  factions and jobs from `getPlayer`, installed augmentations from
  `getResetInfo`, and asks the player for the rest. The docs fill some
  of the gap.
- Installing augmentations kills every script, and the autoexec setting
  only fires on game load. The player re-runs the resident after each
  install, and the proxy's "not connected" answer tells them to.

## Done when

- A scripted playtest passes: a fixed list of newcomer questions ("what
  is this game?", "how do I make money?", "what's a faction?", "what
  should I do now?") asked on a fresh save at minute zero, after the
  first NUKE and after the first faction. Each answer must be right
  against live state, not spoil past the player's progress, give a
  concrete next step, and assume no programming.
- Every helper fits its RAM budget, checked in the game with
  `getScriptRam`.

A real non-programmer tester is worth planning for the release, not for
each increment.

## Rejected along the way

- **Viteburner.** Abandoned since 2023 and pinned to Vite 4. It's a sync
  daemon that replaces filesync, not a bundler.
- **The proxy speaking the Remote File API.** Everything useful needs
  the in-game script running anyway, and the script can do everything
  the RFA can, including RAM calculation. The game also holds only one
  RFA connection, which would fight with filesync.
- **The game owning MCP with the proxy as a filtering pipe.** The
  handshake hangs until the game connects, and the proxy has to parse
  MCP to filter tiers regardless. A split where the proxy handles the
  handshake and passes through the game's tool list depends on clients
  honouring tool-list-changed notifications.
- **One resident script with everything.** It would cost about 7GB all
  the time, and a switched-off tier would still cost RAM.
- **wget from GitHub releases.** The in-game version would follow the
  release, not the proxy the player actually has.

## Open

Experiments, before building on them:

1. Can the in-game script read the docs the game bundles? If so the
   proxy doesn't need to ship them. It may take a bundle hack, and it
   must not use the literal identifiers `window` or `document` (25GB
   each).
2. Does `ns.wget` to `http://localhost` get past CORS and mixed content
   from both the Steam build and `https://bitburner-official.github.io`?
   If the web build blocks it, web players need a fallback install.
3. Does tide still work against TypeScript 7? It speaks tsserver and
   TS 7 speaks LSP.

Consequences to act on:

- The builds become ordered: the TypeScript output has to exist before
  the Rust binary embeds it.
- The proxy checks `Origin` on the websocket and HTTP endpoints, or any
  web page could pose as the game.
- `README.md` and `CLAUDE.md` still describe a dumb pipe and a
  one-file bundle. The game resolves static imports between scripts
  (`NetscriptJSEvaluator.ts`) and RAM calculation follows them, so
  one file was never required, only no dynamic imports.

Next: the dispatcher conversation, then experiments 1 and 2, then agree
the tool schemas and tests. The proxy owning MCP is the costliest thing
to undo; it gets a draft decision once code is about to depend on it.

## Later the same day: the detached remote NS proxy

My dispatcher idea, short version, not up for questions yet:

- A tiny supplier script stores the `ns` it was passed in a module-level
  variable.
- A fully independent task, a function `mcp.js` hands to the JS engine
  to run later, owns the websocket to the Rust server. It loops, parking
  on a promise resolved by a timeout.
- The task never touches a real `ns` directly. It goes through a proxy
  that looks like `NS` but records which functions are called and what
  they cost, and compares that against the RAM the supplier script has.
  When RAM runs low, a new supplier script is spawned to hand over a
  fresh `ns`.

This replaces "thin resident plus short-lived helpers" above. The game
charges RAM per call against the supplier's allocation
(`dynamicRamUsage`, and `ramOverride` at `NetscriptFunctions.ts:1202`),
which is what the recorder has to track.

Naming: the Rust half is the MCP server, not "the proxy". "Proxy" now
means the `NS` stand-in above. README.md, CLAUDE.md and the `proxy/`
directory still use the old name.
