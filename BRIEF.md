# wafflehouse: a brief for the shared playground

A starting point for whoever builds the playground, written 2026-10-30. It replaces the earlier
`../playground-recommendation.md`, which predates the contract: everything a tool exposes is now
defined by **table-manners** v0.1.0 (github.com/markhayden/table-manners, crate `table-manners`
on crates.io). This brief says what the playground is, what it reads from the contract, the
gaps the contract leaves for it, and an order to build in. Treat it as the input to a spec, not
the spec.

`wafflehouse` is a working name, free on crates.io and on GitHub at the time of writing.

## The idea

The waffle tools are command-line programs an agent chains: generate an image, check it with
squint, mask what squint found with rascal, paint inside the mask with a filler, check again.
The playground is the human-driven version of the same chain: one local program with a browser
UI that lists the tools installed, runs any action with a form, shows progress, frames each
tool's own result page, and sends an output of one run into the next tool with one click. It
also keeps a history so any run can be repeated or forked with one change.

It is a peer of the tools, not part of any of them. Each tool has a thin `playground`
subcommand that finds wafflehouse, offers to install it, and starts it. One release of the
playground serves every tool.

## What the contract already gives it

table-manners was written with the playground as one of its two clients (the other is an
agent). Section numbers below are CONTRACT.md's.

| The playground needs | The contract provides |
|---|---|
| what a tool can do, as forms | `<tool> describe --json` and the MCP `describe` tool (3): actions with a `params_schema`, plus, for image tools, image slots with roles, location slots with the kinds they accept, outputs by role (11.2) |
| to run an action | `<tool> serve`: MCP over stdio at revision `2025-11-25`, one tool per action (5) |
| progress on a 40-second paid run | progress notifications at least every `progress_interval_s` (at most 10 s), strictly increasing, with a one-line message (4.1) |
| a stop button | MCP cancellation; the run is then readable with status `cancelled` and spend to date (4.2) |
| history | `<tool>://runs` newest first, including running ones; `<tool>://run/<id>/run.json` and `…/journal` (4.3) |
| one result shape for every tool | the result header (2): status, error code with `retryable`, spend, cache, timestamps, notes; the tool's own `body` beside it |
| images to show and pass on | image outs with a role, a reference and a locator (path, inline or resource) (10.2) |
| findings to draw | locations on the original image in edge fractions, split at a panorama's seam into parts (9) |
| to warn before spending | `cost.may_spend`, `cost.needs_env` by name only, and an `estimate` action on any tool that spends (3, 4.5) |
| to render a tool's own page | `renders`: command templates with `{run.dir}` (3) |
| words to show with findings | `vocabulary`: every disposition, confidence and evidence word the tool emits (11.2) |
| to not care what kind of tool it is | the core and profiles split (1): a core-only tool (text, code) has actions and results with no image fields |

So the playground needs no tool-specific code: everything it shows comes from a description, a
result or a tool's own page. The first time the host knows what a seam or a verdict is, the third
tool costs what the first did.

## The shape

A local program in its own repository, `wafflehouse`, with three parts:

1. **A host** (Rust, one binary, the tools' build mechanics: `just check`, cargo-dist). It finds
   installed tools, starts each one's `serve` as a child process and holds an MCP client session
   to it, turns descriptions into forms, runs actions, relays progress, reads runs back through
   the run resources, and serves the UI on loopback.
2. **A browser UI** served by the host over HTTP and a WebSocket from the first commit. A tool
   picker, a form per action, a run view (the progress stream, then the tool's own page framed
   beside a before-and-after view of the images in and out), the history, and "send to" for
   chaining. Shipped embedded in the binary, with no separate build toolchain required to build
   the host.
3. **The `playground` subcommand in every tool** (squint task 7.9, rascal's conformance
   milestone). It finds `wafflehouse` (`PATH`, then the data directory), offers the install
   command when absent and never runs an installer without a yes, starts it with
   `--tool <name>`, and opens the browser. No UI in the tool.

Because the UI is already a client of a server, hosting it later is a deployment question
(users, storage, secrets), not a rewrite. That is out of scope for the first version.

### Chaining

"Send to" is the reason the playground exists above the tools. It works from roles and kinds in
the descriptions, not from knowledge of either tool:

- an image out with role `result` from rascal can fill any image slot of another tool; the host
  passes it by path with its reference as `expect`, so the receiving tool verifies it is the same
  file;
- a location in squint's `locations.found` can fill a rascal location slot whose `kinds` include
  its kind;
- the history records each run's inputs as references to earlier runs, so a chain shows as one
  lineage and can be replayed.

The mask handoff (rascal's mask going back to squint as "change was allowed here") is a later
milestone of the tools; the playground gets it for free when squint's description gains the
location slot.

## Gaps the contract leaves for the playground

Each of these is a candidate for table-manners v0.2, which is open until rascal's conformance
merges. Raise them in the contract repository rather than working around them in the host.

1. **How a render command reports its address.** `renders` gives a command
   (`squint report view {run.dir} --port 0`), but not how the host learns the URL it serves. A
   rule such as "a render command prints its URL as the first line of stdout and serves until
   stdin closes" would settle it.
2. **Renders on a network transport.** `run.dir` is null off stdio, so `{run.dir}` cannot be
   substituted. Local only is fine for the first version; say so.
3. **Session budget across a long-lived server.** The contract's session budget lasts as long as
   the server process. A playground that keeps `serve` running all day spends the whole budget
   in one session; decide whether the host restarts servers, shows the remaining budget from
   `estimate`, or asks the contract for a reset.
4. **Which tools exist.** Discovery is not in the contract. A list of known tool names on
   `PATH` plus a config file is enough for the first version; a registry is not needed.
5. **Inputs the playground does not hold.** Image slots take paths inside the client's MCP roots.
   The host is the MCP client, so it declares roots; decide which (the user's chosen folders, the
   playground's data directory for uploads).

## What not to build

- **The UI inside each tool's binary.** Several builds of the same screens, version drift, and
  no owner for the screen that shows two tools.
- **A second rendering of a tool's result page.** squint's report viewer and rascal's run page
  are tested in their repositories; frame them.
- **Key storage.** Tools read their own environment. The host passes the environment through and
  shows whether each `needs_env` variable is set, never its value.
- **A node-graph recipe editor.** A run is a form with parameters; chaining is "send to". A graph
  editor is a product of its own.
- **Tool-specific code in the host,** as above.
- **A command-line fallback for tools that do not serve MCP.** Both tools will serve the contract
  over MCP; one transport keeps the host small.

## Order of work

Neither tool conforms yet: squint's milestone M7 and rascal's conformance milestone are both
starting against v0.1.0. The playground can start now without waiting for either.

1. **A fake conforming tool in the playground's own test kit**: a tiny `serve` that answers
   `describe` from table-manners' `examples/description/valid/` documents, runs an action that
   sleeps with progress, honours cancellation, and returns the example results. Every later step
   is tested against it, offline and free.
2. **The host and UI against the fake**: discover, describe, form, run, progress, cancel,
   history from `<tool>://runs`, before and after.
3. **rascal**, as soon as its `serve` conforms: its actions are fast and deterministic, so it is
   the first real tool. Frame its run page.
4. **squint**, after squint's M7 (tasks 7.7a to 7.8): the first long, paid run, where the cost
   notice, `estimate`, progress and the stop button are proved.
5. **"Send to"** between the two: rascal's result into squint's candidate slot; squint's finding
   into rascal's location slot.
6. **The `playground` subcommand** in both tools, and an install path (`cargo install wafflehouse`
   and the release archives).
7. **Later**: the evaluate half (rascal's judging pages, squint's evaluation results) as a mode of
   the playground; hosting.

Each step leaves the playground usable. A tool is "in" when the host can describe it, run an
action, stop one, and show its page.

## Decisions to make before the spec

- The name, and the crate and repository to reserve.
- The UI stack: vanilla JavaScript like both tools' pages, or a framework built ahead of time and
  embedded; the constraint is that `cargo build` of the host needs nothing else.
- The data directory and what the history stores (run references and inputs only, or copies of
  outputs).
- The MCP client: the official SDK, `rmcp` pinned at `=3.8.0` as both tools pin it, with an
  independent test client as the contract requires of servers.
- Which of the five gaps above go to table-manners v0.2 first; gap 1 blocks framing any page.

## Reading list

- `../table-manners/CONTRACT.md`: the contract; `examples/` for valid documents of every shape.
- `../table-manners/MAPPING.md`: where squint and rascal differ from the contract today.
- squint (see [current checkout/source inventory](docs/integration-sources.md#squint)): ADR 0018, SPEC "External interface", PLAN milestone M7; `report view`
  for the page to frame.
- rascal (`../rascal`): `rascal serve` (crates `rascal-mcp`, `rascal-mask/src/serve.rs`), the
  `dash` page, `rascal_testkit::mcp_client` as the model for an independent test client.
- `../playground-recommendation.md`: the earlier version of this brief, for its survey of what
  each tool renders today.
