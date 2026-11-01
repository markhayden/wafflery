# Integration source inventory

## Current v0.3 cutover — 2026-11-01

V02 uses the authorized sibling path dependency, exact version 0.3.0 at
`e503a238ad6cc833834891c2c2cece7152b08bfc` on `../table-manners` branch `v0.3`.
Publication is pending and does not gate V02. Hosted CI checks out this exact revision beside
Wafflery; a coordinator-requested follow-up switches to crates.io `=0.3.0` after publication. The host accepts only live v0.3 and keeps older saved evidence readable.
The independent peer supplies current discovery/offline validation; real Rascal M12 and Squint
M9 remain V18 gates. Previously tested v0.2 executables below are historical evidence, not
compatible live targets for the cutover. No sibling files are modified.

## Previous v0.2 integration — 2026-11-01

- Current sibling checkout paths are `../rascal` and `../squint`; there is no alternate-name lookup.
- The canonical launcher is now `wafflery`, with `WAFFLERY_BIN`. The coordinator owns the matching
  sibling launcher rebuilds. Archived binaries below verify direct MCP serving only; using them
  for the renamed `playground` helpers would not verify the new H1 entry path.

- Published contract: table-manners `=0.2.0`, tag `3c8b55c70f03581757549f67d04b85dc393ad88d`.
- Rascal committed source: `27ef3b5`, isolated `git archive` build; no filler or credentials.
- Squint committed source: `3e6223a`, isolated `git archive` build with `testkit`; scripted provider.
- Both pass the same generic host's session/association checks. Squint estimate journal fix
  `718eec7` is included and its previously failing complete-evidence probe now passes.
- The sibling working trees contain ongoing work and were not modified or built in place.
- Protocol remains 2025-11-25. Tagged v0.2 introduces C4; list image slots and other proposals
  are not part of this tag. Provider readiness, paid caps, native replay and viewer integration
  remain separate work. The observations below are historical source evidence.

## Historical audit — 2026-10-31

Rechecked 2026-10-31 for the first M4 real-tool entry slice. This is source/fixture inspection,
plus Wafflery's explicit real-Rascal coordinator/browser checks, not a fresh run of sibling
test suites. Use repository identity and repository-relative paths below;
do not make links depend on Squint's temporary checkout folder name.

## Table-manners

Repository: [markhayden/table-manners](https://github.com/markhayden/table-manners).
Current checkout: `../table-manners`, HEAD `b260527`, clean during inspection.
Published contract remains `table-manners = "=0.1.0"`; tag baseline `d5a2d8e`.
The checkout contains v0.2 draft schemas and proposal text; v0.2 has not been tagged.
These are not a released replacement for the host's exact v0.1.0 dependency.

Relevant paths: `CONTRACT.md`, `schemas/`, `tests/fixtures.rs`, `fixtures/squint/`,
`PROPOSALS.md`. The fixture test resolves schema URNs through the crate's embedded `SCHEMAS`.
Squint's cancelled fixture exists at `fixtures/squint/run-cancelled.json`.
Proposals cover list image slots (`many`), future MCP roots replacement, unconfigured readiness,
nonretryable authentication failures and renderer readiness. C4 association/session semantics
now include the Rascal/Squint review amendments. Adoption still waits for a tagged contract.

## Rascal

Repository: [markhayden/rascal](https://github.com/markhayden/rascal).
Current checkout: `../rascal`, HEAD `fc819f7` at the later Squint-entry audit, with active
uncommitted work. The completed Rascal integration used `9c52138`. M9 is merged;
the plan records its checks/tag. This audit did not rerun Rascal's full suite. The initial local
binary identified itself as 0.1.0 and emitted contract 0.1.0, without an embedded source revision.
During verification its owner rebuilt that mutable binary against the 0.2 draft; Wafflery
correctly refused it. Final v0.1 checks use an offline build from a separate `git archive 9c52138`
snapshot in Wafflery-owned temporary storage. No sibling working files were changed.

Relevant paths: `crates/rascal-mask/src/{describe,actions,serve,playground}.rs`,
`docs/contract.md`, `docs/adr/0020-table-manners.md`, `PLAN.md` tasks 8.6–8.8.
The current description needs no configuration: with no fillers, actions are `may_spend: false`
and `needs_env: []`. `mask_rect` and `seam` without a filler are candidate offline smoke actions.
Use isolated workspace/run directories and the live description before any execution.

The launcher now searches `WAFFLERY_BIN`, PATH for `wafflery`, then `~/.local/bin`,
and invokes exactly `--tool rascal`. Production installation remains unavailable without an
approved release artifact. Wafflery's `dev-rascal.ts` successfully exercised this actual
entry point with the selected binary and an isolated host-owned data directory.

ADR 0020 keeps native op envelopes outside the shared v0.1 action interface. R3 now has a
supplier milestone, M10; C4 adoption is planned in M11 after v0.2. Scheduled work is not a
claim that these interfaces are already implemented. C2/C3 remain integration follow-ups.

## Squint

Repository: [markhayden/squint](https://github.com/markhayden/squint).
Current checkout: `../squint` (renamed on 2026-11-01). HEAD `fcde474` during the historical inspection,
with active uncommitted work. A local `m7` tag exists; the owner reports M7 pushed/released.
No fetch was performed and the current working changes were not modified or treated as a release.

Relevant paths: `crates/squint-cli/src/serve/mod.rs`, `crates/squint-cli/src/main.rs`,
`crates/squint-core/src/contract.rs`, `crates/squint-cli/tests/serve_check_cli.rs`,
`crates/squint-cli/tests/serve_progress_cli.rs`, and `PLAN.md` 7.9/checkpoint M7.
Five named use-case tools match discovery/result attribution; S1's old naming mismatch is resolved.
`serve` still requires config; list-valued references are currently described under `x-squint`.

`estimate` calls `finished()` and writes a zero-spend run directory visible through resources.
Commit `ae6cfb8` moves the execution permit to checks and adds queued cancellation without a run.
Wafflery need not import estimate/control records into its user-facing history.

Contrary to the supplied review's replay inference, a testkit-only `serve --provider-script`
already exists. `provider_for` selects the ScriptedProvider before credential resolution.
`serve_check_cli.rs` and `serve_progress_cli.rs` use this path. No duplicate implementation
request is needed. Production calls without that test feature still require their configured
provider setup; `replay: None` alone does not establish the testkit path is unavailable.

The initial Squint-entry probe built an isolated `git archive fcde474` with `--offline --locked
--features testkit`. Its CLI offline estimate succeeded, but MCP discovery failed compiling the
embedded `#/$defs/Kind` reference. Scripted checks also still advertise `may_spend: true`.
The host retains its schema and spend guards. See [the focused handoff](handoffs/squint-discovery-blockers.md)
for the historical evidence. [Squint #2](https://github.com/markhayden/squint/issues/2) was then
closed with `a0a2dde`; a fresh isolated archive build of that exact revision passes discovery,
the scripted source-fidelity coordinator test and the two-tool Chromium/WebKit journey. No
contract change or paid-guard bypass was required. The sibling checkout was `15f17ed` and clean
when follow-up work began; its mutable build products were not used. Estimates still complete
without a readable journal, leaving [their evidence incomplete](handoffs/squint-estimate-journal.md).

## Shared unresolved behavior

Both tools pin rmcp 3.8.0 and MCP revision 2025-11-25. Run association is currently encoded in
human progress strings: Rascal `run {id} started (...resource...)`; Squint starts a message with
the resource URI and other text. Wafflery will not parse either format. Budgets are per-process
in-memory ledgers with no shared server-session identity. A restart resets the ledger by design;
it becomes a hidden replenishment problem only if the host restarts without making that boundary
explicit. C4 proposes structured association and session observations, not a durable spend ledger.

## Development environment observations

This agent shell resolves Node through Volta and reports **22.13.1**. The owner's external review
reports **24.18.0**. Treat these as different executable-selection environments; record the actual
Node path/version used in M0 rather than overwriting one observation with the other. Rust 1.99.1
and Bun 1.4.3 match. Chromium and WebKit caches exist under `~/Library/Caches/ms-playwright`;
cache presence does not prove the selected Playwright version can launch them under this sandbox.

Historical review files retain their dated source revisions. Current integration claims should
reference this inventory instead of rewriting historical evidence.


## M3 evidence archives and formatting

- [tar 0.4.46 Builder](https://docs.rs/tar/0.4.46/tar/struct.Builder.html): explicit `append_data`
  entries, fixed safe paths, regular-file headers, and explicit archive completion. No directory walk.
- flate2 1.1.10 local crate source: `write::GzEncoder::finish` completes the compressed stream;
  the writer has an independent byte limit.
- [Prettier CLI](https://prettier.io/docs/cli): scoped `--write` formatting used for the workspace
  and browser tests with ephemeral Prettier 3.6.2. It is not an application dependency.

## Temporary Wafflery mark

`ui/src/assets/wafflery-mark.svg` traces the owner-supplied white isometric waffle with
melting butter (owner attachment, 2026-10-31), replacing the upright
waffle variant. ImageTracerJS 1.2.6 and pngjs 7.0.0 were used in a disposable tooling directory,
following their packaged Node examples. The trace retains the light shapes, removes the black
background, crops empty margins and uses the header foreground color. Neither package is an
application dependency. The original attachment is unchanged.
