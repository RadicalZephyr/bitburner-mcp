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

## Conventions

### Conventions have an owner

Every convention in this repository is owned by one document, and the rest are
restatements. The ADR rules are owned by
[`docs/decisions/README.md`](docs/decisions/README.md); what appears here and in
[`CONTRIBUTING.md`](CONTRIBUTING.md) restates it. Change the owner first, then
every restatement. A restatement that has fallen behind is worse than none,
because it is read with the same confidence as the owner.

Anything normative -- a convention, a gate, a process, anything that changes
what someone does -- has to reach `CONTRIBUTING.md`. Write it there for a human
contributor and here for an agent where the register differs, but never
introduce a rule here that a contributor reading only `CONTRIBUTING.md` would
miss. A rule sequestered in an agent's briefing is a rule the people doing the
work never saw.

### Architecture decision records

Decision records live in [`docs/decisions/records/`](docs/decisions/records/),
one per file, `NNNN-kebab-case-title.md`. They are **living documents** --
edited to stay current rather than frozen on acceptance, because git is the log
and the document is the projection. New information goes in as a **dated
addition marked as arriving after the decision**, never as a silent revision of
the original reasoning. A record whose last row is still `Drafted` is exempt --
it has not made a decision yet, so edit it freely without dating anything, and
the test is the status row rather than whether a pull request is open. Read the
directory as the current state of this repository's decisions.
[`0001-recording-important-decisions.md`](docs/decisions/records/0001-recording-important-decisions.md)
argues for all of it; [`README.md`](docs/decisions/README.md) one level up has
the rules, and where a conflict or trade-off came up it belongs in the section
it concerns rather than a changelog at the bottom.

A record's status is a **dated transition log** in a collapsed `<details>` block
at the top of the file, with the current state in the `<summary>` line:

```markdown
<details>
<summary><strong>Status:</strong> Implemented 2026-11-03</summary>

| Date       | Transition  |
| ---------- | ----------- |
| 2026-09-10 | Drafted     |
| 2026-09-24 | Accepted    |
| 2026-11-03 | Implemented |

</details>
```

Transitions are `Drafted` (replacing a separate `Date` field), `Accepted` (a
commit before the record's PR merges), `Implemented` (the PR that finishes the
work), `Superseded by NNNN`, and `Deprecated`. The last row is the current state
and the summary restates it.

Two things the log is **not**. It is not an edit log -- only state transitions go
in it, and changes to a record's content are dated additions in the body next to
the reasoning they concern. It is not a changelog at the bottom -- it records
state, never reasoning, which is what keeps it consistent with writing conflicts
and trade-offs into the section they belong to. A row that wants a sentence of
explanation means that sentence belongs in the body.

Editing versus superseding: edit while the decision is still the decision; write
a new record when someone following the old one would now do the wrong thing.
Mechanically -- if the change can be a dated addition it is an edit; if it means
deleting a claim someone may have acted on, the old claim earns its own record.

**Any code that produces concrete data used in the argumentation of an ADR must
be committed somewhere a reader can run it** -- a number quoted in an ADR has to
be re-derivable from a checkout, or the record is asserting rather than arguing.
Which of two homes depends on one question -- can a reader run it from a share
link?

- **Needs this project's code** → the sub-project at
  [`docs/decisions/experiments/`](docs/decisions/experiments/), as an entry
  point named after the record (`0001-some-decision`), run with the command its
  README names. A dependency added there goes through the same checks as the
  rest of the repository; not publishing it is no reason to skip that.
- **Needs anything else the playground cannot run** → the same sub-project, on
  the same terms. Most often a dependency the service does not carry: what a
  playground can run is a property of the named service, not of the language, so
  judge it per experiment. Take this route because the playground cannot run the
  thing, not because adding a dependency locally is quicker. The dependency
  leaves when the experiment does -- struck from the manifest in the same commit
  on the delete exit, deliberately adopted as a permanent dependency of the
  benchmark or test suite on the promote exit.
- **Needs only what the playground can run** (the toolchain, its standard
  library, and any dependency the service supplies) → a share link on the
  playground named in `docs/decisions/README.md`, recorded in the ADR. This is
  also the only route for a build-time experiment: a case that must _fail_ to
  compile or type-check cannot be an experiment entry point, because a
  sub-project that does not build breaks the project's build. If the README
  names no playground, everything that builds goes in `experiments/`, and a
  must-fail case is recorded in the ADR as the four parts below with the
  provenance line cut to `TOOL VERSION (released DATE)` and no link -- as is a
  must-fail case needing something the playground cannot supply.

**Develop the experiment locally and mint the link last.** Run it with the
local toolchain against a file in your scratch directory until it produces the
result the record will quote, and only then create the share link. Never mint
one to find out what happens, and never rework an experiment after minting --
the superseded links stay live and nobody can collect them. **At most one share
link a minute**; if a record needs more than that, ask the user to mint them
rather than minting them faster.

A playground record is four parts in a fixed order: a bolded label saying what
it demonstrates, the source in a code block, its output in a `text` block, and a
provenance line as a blockquote beneath them --

```text
> TOOL VERSION (released DATE) - output checked DATE - [PLAYGROUND](URL)
```

Keep the fences bare; the blockquote is only the provenance line, which renders
muted. Both dates are load-bearing: the released date belongs to the toolchain
that produced the quoted output, the checked date is when the link was last
confirmed to still produce it. **The version stamp is required and is checked in
review** -- a link that names a channel rather than a version drifts, and
without the stamp a reader cannot tell whether the toolchain moved or the record
was wrong. `docs/decisions/README.md` has the skeleton.

An experiment is maintained while the decision it serves is still being argued
or built, and leaves the moment that decision is done -- which the record dates
exactly, in the `Implemented` row of its status log, or in the `Deprecated` row
if the decision was withdrawn rather than built. It goes by one of two exits --
**deleted** (it measured internals the ADR replaced; an entry point that no
longer builds is deleted, not repaired, and the ADR cites the commit that
produced its numbers) or **promoted** (it still answers a live question, so it
stopped being research: a measurement worth re-running moves to the benchmark
suite, a property we promise moves to the test suite). It never lingers.

So do not fix up an experiment that the project's checks break on without first
checking whether its record has logged `Implemented` or `Deprecated`; most
records argue for changing the internals the experiment was measuring, and
breaking is the expected end of its life. The sub-project is a staging area,
not an archive, and should trend toward empty.

### Research is evidence, not a test

**Do not write tests to motivate an ADR.** A test written against the structure
an ADR exists to replace has to be rewritten when the change lands: it guarded
nothing and only enlarged the diff. Support the argument with an experiment in
`docs/decisions/experiments/`, and let the existing suite keep checking that
behaviour did not change while the internals did.

[`CONTRIBUTING.md`](CONTRIBUTING.md) states this for human contributors, and
carries process that never appears here. Read it before changing how anything
in this repository is done -- it is not a restatement of this file, and treating
it as one is how a convention ends up in only one of the two.
