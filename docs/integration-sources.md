# Integration source inventory

## Current v0.4 cutover — 2026-11-02

Live execution resolves published crates.io `table-manners =0.4.0` with Cargo.lock. The packaged
schemas and contract/source files match reviewed tag `v0.4.0` at `d1c49a7`; the package deliberately
excludes fixtures. There is no sibling pin, second contract dependency or live-version fallback.
Older saved evidence remains readable. The independent peer is the required offline check target.

Approved M10 integration snapshots are Rascal `eed6397` and Squint `6dd1e3a`. Both were built from
isolated committed-source archives with locked dependencies, `testkit` and a separate target
directory. No sibling files were modified. All four M5.03 opt-in preflight checks pass through the
v0.4 host with explicit `--allow-dir` grants and isolated XDG folders:

- Rascal offline seam: contract 0.4.0, complete zero-spend evidence, original bytes unchanged, and
  both report kinds open and close. Its scripted filler also passes cap/budget, estimate binding,
  selected-config freeze and restart/new-ledger checks using synthetic charges only.
- Squint scripted reference-consistency: contract 0.4.0, ordered/repeated reference evidence,
  zero spend, original bytes unchanged and both report kinds. Default/selected configuration and
  missing-credential estimates pass without allocating a run or calling a provider.

The same checks pass after composed parameter/callable namespace validation and contextual refusal
adoption. These are current task/setup/report preflight checks, not certification of operation
chains or invocation bundles; M5.21 owns those real-peer journeys. No credentials or paid-provider
calls were used. The V18 results below describe the earlier v0.3 build; their evidence is retained
in [checkpoint V18](plans/v03-real-peer-checkpoint.md).

### V18 real-peer runs and reports — 2026-11-02

Approved snapshots: Rascal `9ee6bc8` and Squint `54b1f41` (superseding `db5cf78` after its viewer CSP fix), built from isolated archives with
`testkit`, locked dependencies and a separate target directory. Sibling worktrees are untouched.
Both declare contract 0.3.0. Probes use disposable libraries and isolated XDG config/data/cache
roots through the host's fixed environment/PATH policy; no provider credentials are supplied.

- Rascal: offline `seam`, zero spend, verified output mask/journal and original bytes unchanged.
  Its declared `dash` server and `page` document both serve framed HTML and close gracefully.
- Squint: scripted `reference_consistency`, zero spend, complete input evidence and a pass fixture.
  Three references preserve order, including one repeated asset copied to distinct execution paths.
  Its declared `report` server and `export` document both serve framed HTML and close gracefully.
- Each report uses the serving executable, the exact frame ancestor and a separate origin. Closing
  removes frame access; the saved attempt is unchanged. Explicit probe failures cannot become skips.
- These checks establish host interoperability, not provider quality or paid-provider readiness.
  Browser acceptance is recorded in checkpoint V18. Estimates are no longer
  actions or runs; the previous v0.2 estimate-journal test has been retired.

### V18 setup and cost controls — 2026-11-02

The same approved snapshots pass real-peer coordinator setup and cost checks under the fixed
PATH and isolated lookup directories. No tool-specific production code was added.

- Rascal without selected configuration describes its seam as ready and free. With a selected
  configuration, its own local `fake-filler` executable reports a synthetic $0.04 cost. A cap above
  the configured limit fails; a lower cap refuses; the accepted quote binds the actual call and
  is preserved with spend evidence. The next call refuses the exhausted session budget. Editing
  the configuration does not change the running session's price/identity; explicit restart does,
  resets the ledger, invalidates old preparations and leaves saved evidence unchanged.
- Squint discovers missing configuration through both default `./squint.toml` lookup and an
  explicit path containing spaces. Adding configuration requires restart. A real Anthropic
  profile without its credential produces a priced readiness refusal, with no host attempt,
  tool run directory or spend; restart invalidates that quote. `ANTHROPIC_API_KEY` is explicitly
  unset for this probe. There is no provider dispatch or credential validation.

This verifies the shared host's cost/setup boundary, not live provider billing or model quality.
Actual provider 401/403 responses and billed calls remain outside this offline acceptance.

### Rascal explicit paths — 2026-11-01

Coordinator-selected Rascal main `9ee6bc8` introduced `serve --allow-dir <absolute-directory>`
for caller paths while declaring contract 0.3.0. M5.02 moves this from Rascal-only development
arguments to the shared host launcher: every registered tool receives its explicit `allow_dirs`
plus the dedicated `Library/execution` staging directory. Development descriptors declare separate
`evidence_dirs` for returned image/report reads. No real registration is rewritten; back up and
explicitly replace old `roots` registrations using the README procedure. The approved v0.4 peers
are Rascal main `eed6397` and Squint main `6dd1e3a`; live structural schema validation now accepts v0.4.

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
