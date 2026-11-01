# v0.3 execution checkpoints

The owner approved V01–V18 on 2026-11-01. Implement in PLAN order, with tests first, accurate docs,
short branches, the full matrix under the shared heavy-check lock, and local merges after review.
The coordinator scans and pushes; no sibling edits, tags or compatibility shims.
The browser correction at 51d7935 passed hosted run
[41358441245](https://github.com/markhayden/wafflery/actions/runs/41358441245); the CI gate is green.

## V01: independent fixtures before live cutover

- V01a: archive the eleven schemas from table-manners e503a238ad6cc833834891c2c2cece7152b08bfc
  with provenance/license; add owned descriptions, sessions, estimates and refusal fixtures,
  including two tool names, all readiness states and estimate refusal variants. Validate with
  an independent JSON Schema retriever; malformed readiness, absent positive-cap declarations
  and malformed estimates must fail. Keep the live host's exact v0.2 guard and paid guard.
- V01b: expose those fixtures and scripted estimate control over the existing independent
  line-delimited peer. Test initialize/discovery, session consistency, input refusals and no run
  allocation/spend. No host SDK or tool-private code in the peer. Describe/estimate only: future
  run behavior is added in the owning task, not claimed as implemented here.
- Each slice carries its tests and testkit docs. Validate the joined V01 checkpoint with the full
  matrix, review and then merge locally after hosted CI for the prior fix is green.

V02 uses the authorized temporary sibling path through its merge and hosted checkpoint.
CI checks out reviewed e503a23; the coordinator requests the crates.io switch after publication.
V01’s archived schema input avoids changing the live validator prematurely. Remove the archive in V02 when the dependency supplies the same schemas.

Rollback newest dependent commits first. V01 changes only disposable peer/test coverage; no real
library migration, provider calls or host dispatch behavior. Later rollback checkpoints remain
those in PLAN.md.

## V01 verification

Both test slices were observed failing before implementation: null fixture objects fail the
reviewed schema, and the old peer neither describes v0.3 nor implements estimate. The completed
fixtures and wire probes pass, including actual payload schema validation, before/after ledger
identity, zero spend/reservations, no runs/files, fresh serving sessions, reserved-name exclusion,
readiness/cap/budget estimates and invalid-argument refusals. The runtime still rejects v0.3.

Implementation and test coverage passed review. Under the shared heavy-check lock: formatting,
clippy, 77 Rust tests (three existing opt-in ignores), UI/script typecheck, 12 UI unit tests,
two asset-tooling tests, Storybook, byte-identical clean UI builds, release build and 70 browser
cases passed; four existing real-tool browser cases were skipped. Generated assets/lockfiles
remained unchanged. No provider calls, sibling edits, push or tags.

## V02: coordinated validation cutover

Use the authorized temporary path dependency at reviewed e503a23 during development; switch to
published `=0.3.0` in the coordinator’s post-publication follow-up. Keep MCP 2025-11-25 and the
no-paid dispatch guard.
Accept only v0.3 live descriptions/results/sessions, require estimate discovery for paid actions,
remove cost.needs_env, and retain generic reading of older saved evidence. V03 owns the richer
readiness UX; V07 owns list-input editing and dispatch.

The approved larger-file exception covers this coordinated change. Enumerated sites:
Cargo.toml/Cargo.lock; src/tools/{mod,client,session}.rs; testkit/{main,fixtures,scenarios,
v03_control}.rs; tests/{mcp_client,contract_fixtures,v03_fixtures,v03_wire,attempt_storage,
tool_session_api,rascal_smoke,squint_smoke}.rs and tests/support/v03_schema.rs; UI api/{workspace,
tool-session}.ts, features/actions/{model,ActionForm.stories}.tsx/ts, workspace/{ToolSetup,
ToolSetup.stories}.tsx, affected action/session unit tests and tool-setup/real-tool browser tests;
README, ENGINEERING, integration-sources, foundation-runtime and testkit docs. The coordinator’s
merge-with-path ruling also requires a pinned sibling checkout in .github/workflows/ci.yml.
Remove the generated V01 schema archive/license when the dependency supplies it. Commit generated UI assets with the
UI type/copy change. Additional boundary coverage is in ui/tests/unit/workspace-contract.test.ts and
ui/tests/browser/workspace.spec.ts: the latter reopens a stored v0.2 record after restart.

Tests first: discover unconfigured v0.3 peers with arbitrary names, require reserved estimate
without exposing it as an action, reject live v0.2 and missing-cap/estimate declarations, preserve
v0.1/v0.2 saved evidence, and continue current offline execution. Review the schema/parser/fake
change together; run the full matrix under the shared lock. Roll back this coherent cutover
commit rather than retaining dual-version dispatch. No data migration is required.

## V02 verification

Live descriptions, results and sessions now require exactly v0.3. Paid declarations must expose
reserved estimate and a strictly positive-cap parameter; the no-paid dispatch guard remains.
The fake’s normal execution and control probes use the same exact schema dependency, replacing
the temporary archive. Browser types use readiness requirements, not needs_env. Older saved
v0.1/v0.2 records remain readable; Chromium and WebKit reopen and inspect v0.2 evidence after a
host restart. No real provider calls or data migration occurred.

New Rust discovery and browser boundary tests were observed failing before the cutover.
Targeted checks passed after implementation. The full matrix under the shared heavy-check lock
passed: formatting, clippy, 79 Rust tests (three opt-in ignores), typecheck, 14 UI unit tests,
two asset-tooling tests, Storybook, two byte-identical clean UI builds, release build, and 70
Chromium/WebKit cases (four real-tool opt-in skips). Generated assets and lockfiles stayed stable
after regeneration. An initial CI-mode check stopped correctly on stale assets; the rerun built
them first. Typecheck caught an incomplete new test fixture, corrected before the passing matrix.
Implementation and the incremental CI/docs change passed independent review. The workflow YAML
parses and its exact contract revision exists publicly; hosted execution is for the coordinator
to verify after push. No sibling edits, push or tags.

The coordinator explicitly approved merging with the temporary path dependency. CI checks out
`e503a238ad6cc833834891c2c2cece7152b08bfc` alongside Wafflery without persisting credentials.
After publication, switch the dependency to crates.io `=0.3.0`, refresh Cargo.lock and remove
those temporary checkout/move steps. This later change does not block V03 onward.

## V03: readiness and refresh slices

Implement the approved V03 behavior in three named slices so changes stay reviewable:

- V03a, `test: script readiness changes and optional setup`: add an independent readiness
  scenario and held variant, schema-valid requirement states, and a missing optional filler
  that refuses only calls asking for it. Wire tests prove the no-run refusal releases the ID.
  Sites: testkit/main.rs, testkit/readiness.rs, tests/fake_wire.rs and testkit/README.md.
- V03b, `feat: refresh setup and preserve not-ready refusals`: add authenticated description
  refresh without an execution permit. Preserve a validated -32602 not_ready response as a
  terminal rejected attempt with no fabricated result/run; release local materialized inputs
  and the execution reservation. A prior run association or malformed refusal stays uncertain.
  Sites: coordinator.rs, coordinator/readiness.rs, host/run_api.rs, tests/tool_readiness.rs,
  ENGINEERING.md. V08 retains the broader duplicate/error/cancellation classification.
- V03c, `feat: explain action readiness in tool setup`: show all requirement kinds and distinct
  states/setup prose, refresh description/session while preserving the draft, and keep free
  needs_setup calls available so their arguments remain authoritative. Sites: ToolSetup and its
  stories, Workspace, api/workspace, browser/tool-setup tests. Reuse existing drawer/styles.

Each slice gets failing regression coverage, targeted checks and review before its commit.
The full matrix under the shared lock gates V03's local merge. No setup prose is executed and
no credential values are read or saved by this task. Roll back V03c, V03b, then V03a; no database
migration. Paid quote invalidation and broader refusal retry behavior remain V11/V13 and V08.

V03b review identified concurrent refresh publication and preflight/write ordering as one shared
boundary. Include dispatch_transport.rs in this slice: a per-tool discovery guard moves with the
persisted dispatch permit through the actual write, then releases before provider execution.
A transport gate regression verifies both sides of that boundary without timing sleeps; the
held-peer API test verifies refresh remains responsive during execution. This adds one authored
file to V03b’s approximate five-file budget and keeps the correction in its owning behavior.

V03b targeted verification passed: authenticated refresh during a held run, unchanged saved
attempt declarations, optional not_ready refusal followed by a corrected free attempt, strict
refusal-shape unit checks, and the controlled pre-write/post-write discovery-guard regression.
The latter failed before guard retention and passes without timing sleeps. All library tests and
all-target clippy passed. Independent review approved the corrected concurrency boundary.

V03c also adjusts ToolSetup.module.css to separate requirement entries, with Refresh beside the
action heading so long setup lists do not hide it. Readiness responses never directly patch a
declaration: event/run/cleanup snapshot reads share a serialized queue. A delayed refresh
response was observed restoring stale readiness in Chromium and WebKit; the two-tab regression
now verifies newer state remains after that response is released.

## V03 verification

Readiness refresh, strict no-run refusal handling and the UI passed independent review.
The shared-lock matrix passed formatting, clippy, 84 Rust tests (three existing opt-in ignores),
typecheck, 14 UI unit tests, two asset-tooling tests, Storybook, identical clean UI builds,
and the release build. Browser coverage passed 78 Chromium/WebKit cases with four existing
real-tool opt-in skips. The final browser/typecheck rerun corrected a test locator that matched
both result guidance and its notification; no product change or timeout/retry adjustment was
needed. The requirements screenshot was visually inspected. Generated assets and lockfiles
remained stable. Paid calls remain disabled; no provider calls, sibling edits, push or tags.

## V04a verification

The minimal child environment is implemented and reviewed. Four tests failed against an empty
selector before implementation, then passed: HOME/XDG/locale lookup with fixed PATH, declared
credential and non-secret env selection, excluded/non-identifier names without lookup or base
override, and real child configuration lookup with spaced paths. Values remain in the process
boundary only; no registration or evidence fields were added. Startup currently supplies only
the base environment; V04b supplies names discovered by the separate probe. README and ENGINEERING
record this boundary and the need to relaunch the host after external environment changes.

The full matrix passed under the shared lock: formatting, clippy, 88 Rust tests (three opt-in
ignores), typecheck, 14 UI unit tests, two asset-tooling tests, Storybook, two byte-identical clean
UI builds, release build, and 78 browser cases (four real-tool opt-in skips). Assets and lockfiles
remained unchanged. No provider calls, sibling edits, push or tags.
