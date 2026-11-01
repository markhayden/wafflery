# Integration source inventory

Rechecked 2026-10-31 for the foundation-plan review. This is source/fixture inspection, not a
fresh run of sibling test suites. Use repository identity and repository-relative paths below;
do not make links depend on Squint's temporary checkout folder name.

## Table-manners

Repository: [markhayden/table-manners](https://github.com/markhayden/table-manners).
Current checkout: `../table-manners`, HEAD `8e4d0c3`, clean during inspection.
Published contract remains `table-manners = "=0.1.0"`; tag baseline `d5a2d8e`.
`PROPOSALS.md` contains draft v0.2 work, not a released or accepted replacement schema.

Relevant paths: `CONTRACT.md`, `schemas/`, `tests/fixtures.rs`, `fixtures/squint/`,
`PROPOSALS.md`. The fixture test resolves schema URNs through the crate's embedded `SCHEMAS`.
Squint's cancelled fixture exists at `fixtures/squint/run-cancelled.json`.
Proposals cover list image slots (`many`), future MCP roots replacement, unconfigured readiness,
nonretryable authentication failures and renderer readiness. C4 association/session semantics
are not yet among them; Wafflehouse prepares a separate proposal for the owner.

## Rascal

Repository: [markhayden/rascal](https://github.com/markhayden/rascal).
Current checkout: `../rascal`, HEAD `f0e45fb`, clean during inspection. M8 is merged; annotated
tag `m8` resolves to commit `ebb773f`, an ancestor of HEAD. Do not confuse the tag object ID
with its target commit. The plan records M8 checks passed; this audit did not rerun them.

Relevant paths: `crates/rascal-mask/src/{describe,actions,serve,playground}.rs`,
`docs/contract.md`, `docs/adr/0020-table-manners.md`, `PLAN.md` tasks 8.6–8.8.
The current description needs no configuration: with no fillers, actions are `may_spend: false`
and `needs_env: []`. `mask_rect` and `seam` without a filler are candidate offline smoke actions.
Use isolated workspace/run directories and the live description before any execution.

The launcher currently searches `PLAYGROUND`, PATH for `playground`, then
`$XDG_DATA_HOME/waffle/bin` or `~/.local/share/waffle/bin`, invokes `--tool rascal`, and exits 69
when absent without an installation offer. This is implemented, not unfinished task 8.7.
It needs alignment with the proposed H1 canonical Wafflehouse launcher.

ADR 0020 explicitly keeps native op envelopes outside the shared v0.1 action interface. No
R3 delivery milestone is scheduled in the inspected PLAN. C2/C3 and optional R2 also have no
committed delivery schedule. Wafflehouse cannot treat an unscheduled dependency as imminent.

## Squint

Repository: [markhayden/squint](https://github.com/markhayden/squint).
Current checkout: `../dejaview`; when locating it, check `../squint` and then `../dejaview`, and
verify repository identity rather than assuming both exist. HEAD `eef1f10`, clean at inspection;
15 commits ahead of the locally recorded `origin/main`, no `m7` tag, checkpoint unticked.
No fetch was performed: ahead/behind describes local remote-tracking state, not a fresh remote
query. M7 server work is merged locally; do not call the milestone released.

Relevant paths: `crates/squint-cli/src/serve/mod.rs`, `crates/squint-cli/src/main.rs`,
`crates/squint-core/src/contract.rs`, `crates/squint-cli/tests/serve_check_cli.rs`,
`crates/squint-cli/tests/serve_progress_cli.rs`, and `PLAN.md` 7.9/checkpoint M7.
Five named use-case tools match discovery/result attribution; S1's old naming mismatch is resolved.
`serve` still requires config; list-valued references are currently described under `x-squint`.

`estimate` calls `finished()` and writes a zero-spend run directory visible through resources.
Its call passes through the server semaphore, like checks/describe. Wafflehouse need not import
these control records into its user-facing history.

Contrary to the supplied review's replay inference, a testkit-only `serve --provider-script`
already exists. `provider_for` selects the ScriptedProvider before credential resolution.
`serve_check_cli.rs` and `serve_progress_cli.rs` use this path. No duplicate implementation
request is needed. Production calls without that test feature still require their configured
provider setup; `replay: None` alone does not establish the testkit path is unavailable.

## Shared unresolved behavior

Both tools pin rmcp 3.8.0 and MCP revision 2025-11-25. Run association is currently encoded in
human progress strings: Rascal `run {id} started (...resource...)`; Squint starts a message with
the resource URI and other text. Wafflehouse will not parse either format. Budgets are per-process
in-memory ledgers with no shared server-session identity. A restart resets the ledger by design;
it becomes a hidden replenishment problem only if the host restarts without making that boundary
explicit. C4 proposes structured association and session observations, not a durable spend ledger.

## Development environment observations

This agent shell resolves Node through Volta and reports **22.13.1**. The owner's Claude review
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

## Temporary Wafflehouse mark

`ui/src/assets/wafflehouse-mark.svg` traces the owner-supplied waffle-in-a-hammock image
(2026-10-31). ImageTracerJS 1.2.6 and pngjs 7.0.0 were used in a disposable tooling directory,
following their packaged Node examples. The two-color trace removes the white background,
crops empty margins and uses the header foreground color. Neither package is an application
dependency. The original attachment is unchanged.
