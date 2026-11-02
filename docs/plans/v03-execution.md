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

V02 originally used the authorized temporary sibling path through its merge and hosted checkpoint.
The publication follow-up below replaces it with crates.io =0.3.0 and removes the CI checkout.
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

## V04b execution slices

- V04b1, `feat: select tool configuration paths`: optional absolute config path in registration,
  reject duplicate --config arguments, pass the path as separate argv without reading contents.
  Sites: registration/client, registration tests, README. Adding the optional Rust field also
  requires mechanical `config: None` in six existing test descriptor constructors; review these
  separately from behavior changes.
- V04b2, `feat: replace stopped tool registrations explicitly`: `--replace-registration` with
  `--register`, atomic replacement of existing IDs only, unchanged stopped-host lock. Sites:
  registration, launch options, main/help, registration and CLI tests, README.
- V04b3, `feat: probe requirements before serving`: independent fake startup probes and final
  environment discovery; close the probe before starting the serving process, retain ownership
  on cleanup failure, show added requirements as incomplete setup without bootstrap loops.
  Sites: probe helper/client/coordinator/readiness/environment wiring, testkit startup scenario,
  integration tests, ENGINEERING and testkit docs.

Each slice gets failing-first targeted checks and review. The joined V04b full matrix gates its
local merge. Roll back newest slices first; registration's optional config path is additive,
but there is no alternate invocation or legacy environment fallback.

## V04b verification

Configuration-path and replacement slices each passed failing-first checks and review.
The independent startup tests failed before probe integration, then passed for explicit paths,
HOME/XDG defaults, missing variables, additional serving requirements and failed discovery.
The unresponsive-child regression now also verifies ownership of a retained probe and that no
serving declaration or action exists. Review found no blocking issues.

The host never parses config contents. The fake alone reads synthetic config; its startup log
records names and session IDs without environment values. Only the final serving session is
exposed, and refreshing cannot erase the incomplete-setup notice.

The full matrix passed under the shared lock: formatting, clippy, 98 Rust tests (three opt-in
ignores), typecheck, 18 UI unit tests, two asset tests, Storybook, byte-identical UI builds,
release build and 78 browser cases (four real-tool skips). Assets and lockfiles stayed unchanged.
No provider calls, sibling edits, push or tags.

## V05a execution boundary

Keep each entry's owned client and declaration in one swappable state, retaining its registration
for repeated probing. Active attempts retain their original client; cancellation and reaping never
look up a newer generation. Serialize replacement with submission and discovery, reserve the tool,
close the previous child before probing, and publish the replacement atomically. A rejected or
failed restart never silently forces a child. A session-specific revision rejects stale drafts.

Tests cover authenticated confirmation, submit/restart ordering, active/stuck children, failed
startup and explicit recovery, and unchanged historical evidence after an exited unknown run.
The execution worker must finish cleanup before a reaper releases its reservation. Files include
coordinator/readiness/shutdown wiring, a restart helper, revision binding, API route and tests;
this lifecycle change necessarily spans those entry points. Rollback: revert V05b first if landed,
then V05a; existing registrations and saved attempts remain readable.

V05a's initial five tests failed at the absent restart endpoint, then passed. Review found that
cancelling a requester could abandon a replacement before publication. An initialization gate
reproduced the abandoned replacement; the coordinator-owned lifecycle now survives cancellation,
and the regression passes. Follow-up review approved the fix. Edited-config coverage verifies
the old serving process exits before a fresh probe and that the new serving environment follows
the edited requirements. Twenty repeated submit/restart races passed without retries.

The first full browser pass failed one existing WebKit setup test: the label was empty after
refresh. Its trace shows the fill never established a value before opening setup; all observed
state responses had the same revision. Forty diagnostic repeats passed. The exact WebKit
insertion mechanism remains unconfirmed; no application reset was demonstrated. The test now
clicks the field, checks focus, fills once and verifies the value before testing preservation.
There are no refills, retries or longer timeouts; temporary diagnostics were removed. This
observable-precondition adjustment passed review.

The final matrix passed under the shared lock: formatting, clippy, 105 Rust tests (three ignored),
typecheck, 18 UI unit tests, two asset tests, Storybook, byte-identical UI builds, release build
and 78 browser cases (four real-tool skips). The focused setup test also passed 40 repeats
with the observable input precondition. Lockfiles and embedded assets were unchanged.

## V05b execution boundary

Place deliberate restart in the existing setup drawer. Keep its controls mounted independently
of a missing declaration so failed replacement can be retried explicitly. The action explains
that a new session has a fresh spending ledger and preserves prior unknown charges/history.
Require confirmation, disable while active, and never trigger restart from refresh or an error.
Invalidate setup observations on the published session ID, including in another attached tab.
Keep saved evidence and draft values separate from the replacement lifecycle.

Browser coverage: cancel without a request, one confirmed restart, two-tab session identity,
draft preservation, no action execution, active-work refusal, failed restart and deliberate
recovery. Stories cover idle, busy and disconnected states. Rollback restores the existing
refresh-only drawer; V05a's authenticated restart API can remain independently usable.

The initial restart browser cases failed against the missing controls. With implementation, all
24 setup cases pass across Chromium/WebKit, including confirmation focus, two-tab identity and
320/768/1440-pixel layouts. Reviewed desktop and small-screen screenshots. Review approved; local
per-tool pending state also disables restart after closing/reopening setup before its busy
snapshot arrives. Controls remain available after a failed replacement without a declaration.

V05b full matrix passed under the shared lock: formatting, clippy, 105 Rust tests (three ignored),
typecheck, 18 UI unit tests, two asset tests, Storybook, byte-identical UI builds, release build
and 86 browser cases (four real-tool skips). Embedded UI assets were regenerated and verified.
No provider calls, sibling edits, push or tags.

## Hosted V05a setup-test follow-up

Hosted run 41368632787 failed an existing unavailable-tool browser test. The trace showed no
second run submission and no restart: a required label remained empty, then the assertion timed
out and success-only route cleanup was skipped. The fix triggers invalidation through Refresh,
waits for the mutated declaration response, and drains routes in finally. Review also required
arming mutation only after the explicit refresh request.

The final test passed 40 repetitions, then the complete matrix passed with 105 Rust tests, 18 UI
unit tests, two asset tests and 86 browser cases (three Rust ignores/four browser skips), plus
formatting, clippy, typecheck, Storybook, asset reproducibility and release build.
[Evidence and diagnosis](../reviews/2026-11-01-setup-refresh-ci.md). No production change was
needed for this failure. V05b was already locally merged before the hosted V05a failure arrived;
its full matrix passed separately. Resume V06 after the hosted gate is green.

## V06a execution boundary

Bind `many: true` slots as non-empty ordered arrays; scalar slots remain objects on the wire.
An absent optional slot is omitted; an explicitly supplied empty list is refused. Preserve
duplicate identities as separate items/copies and retain each item's frame and role.

Before copying any input, check all slot/cardinality/frame bindings and metadata: at most 32
images and 512 MiB per attempt, with the existing 128 MiB per-file limit. Repeated identities
count per occurrence because each is materialized separately. Byte reads still verify immutable
fingerprints; metadata checks do not replace that verification. Extract binding into a helper
and metadata lookup from asset reads so preflight does not load all bytes into memory.

Tests exercise wire shape/order, duplicates, optional omission/empty-list refusal, invalid scalar
and frame counts, aggregate limits, corrupt stored bytes and cleanup. Cleanup validates every
path before deleting anything, including array elements, and leaves unrelated files untouched.
The independent peer declares and consumes a list slot; V06b owns complete result-list evidence
association, so V06a does not claim that later checkpoint. Files include binding/coordinator
wiring, asset metadata lookup, cleanup, peer fixtures and tests. Rollback V06a after V06b/V07 if
those have landed; existing scalar evidence and library records remain readable.

V06a verification: the binding regressions failed first on scalar-only handling, empty-list
acceptance and missing aggregate preflight. Cleanup regressions then exposed remaining array
copies. Six binding tests and two cleanup tests now pass. Review found no implementation defect
and requested explicit per-occurrence frame coverage; that test and mismatched list-frame
counts were added and pass. The full matrix passed with 112 Rust tests (three ignored), 18 UI
unit tests, two asset tests and 86 browser cases (four skipped), plus fmt, clippy, typecheck,
Storybook, asset reproducibility and release build. The additional frame test passed afterward
alongside all six binding tests, formatting and all-target clippy, bringing Rust coverage to 113.
No UI assets, dependencies or contract changes. Result-list evidence remains V06b.

## V06b execution boundary

Result association checks every reported image against the submitted slot, role and frame.
Completed results must account for every occurrence in each slot, in order. A non-completed
result may report an ordered subsequence, including no images; missing occurrences produce a
preservation notice without replacing the tool's outcome. Repeated identical entries are counted,
not assigned inferred positions. Images-profile containers remain required for all outcomes.
Unit regressions cover adversarial identity/order/role/count changes; the independent peer and
host integration cover complete lists and partial non-completed evidence. Roll back V07 before
V06b if needed; V06a's saved bindings remain readable.

V06b verification: all three new association tests failed before implementation. Seven binding
integration tests now include complete-list preservation and six partial/empty terminal cases.
The first full matrix exposed stale cancellation assertions that called absent input evidence
complete; those now require incomplete evidence and the completed session observation. Review
found no blockers. The final full matrix passed: 117 Rust tests (three ignored), 18 UI unit tests,
two asset tests and 86 browser cases (four skipped), plus fmt, clippy, typecheck, Storybook,
asset reproducibility and release build. No product UI, dependency or contract changes.

## V07 execution slices

V07a owns pure selection encoding/restoration and its unit regressions. V07b wires those rules
into the existing image editor, with coordinates carried on each selected occurrence instead of
one projection per slot. The component exposes an item-details slot for coordinate controls;
reordering moves the entire selection object. Examples and history restore immutable saved
inputs. Scalars retain their existing labels and behavior; optional empty lists are omitted.
Full matrix and review gate each slice. Roll back V07b before V07a; no storage migration.

V07a verification: ordered-list encoding failed first against scalar-only cardinality. Four
additional unit tests cover list omission/limits, duplicate occurrences with distinct coordinates,
reorder/removal and restoration. Review found no blockers. The initial full matrix exposed an
existing WebKit held-run test that submitted without establishing a required label value;
[the test correction](../reviews/2026-11-01-held-run-input.md) passed 40 focused repetitions.
The final full matrix passed: 117 Rust tests (three ignored), 22 UI unit tests, two asset tests,
86 browser cases (four skipped), fmt, clippy, typecheck, Storybook, byte-identical UI rebuilds and
release build. V07b owns visible per-item coordinates and saved-workspace restoration.

## V02 publication follow-up

The coordinator confirmed table-manners 0.3.0 publication. Adopt its exact registry version and
checksum, remove CI's temporary sibling checkout, and update active runtime/source documents.
Published source is 9ac2142bf9315f4c5a0eb5935aa94da9bc95a697; all eleven schemas match the reviewed
e503a23 and current sibling checkout byte-for-byte. This is a dependency-source change only;
no schema, protocol or storage changes. Full matrix and review gate the local merge.

Publication verification: Cargo.lock changed only the table-manners registry source and checksum
(9bec37d99d246d9b9d2a05da9c9be30ca3951773e3f762c3b8f5c14fa2360c79). Offline all-target checking
passed. Review corrected README's remaining sibling-checkout instruction; final review found no
blockers. Full matrix passed against the published crate: 117 Rust tests (three ignored), 22 UI
unit tests, two asset tests and 86 browser cases (four skipped), plus fmt, clippy, typecheck,
Storybook, asset reproducibility and release build. No other dependency or generated asset drift.

## V07b editor verification boundary

ImageInput exposes per-item details for coordinate controls; ActionForm derives scalar/list
limits from the declaration. Coordinates travel on selected occurrences. Workspace uses the
selection encoder, restores saved inputs from history/examples, and appends saved assets to lists.
Unknown saved coordinate systems are refused rather than silently reinterpreted. The input pane
and Run footer retain their existing layout. A list story and browser journeys cover reorder,
removal, optional omission, history and example restoration, duplicates with distinct frames,
and 320/768/1024/1440 layouts.

Review found pending imports could overwrite newer insertions or a restored draft. A delayed
upload reproduced the restored list growing from one item to two. Imports now retain their
editor ownership, append to the latest committed selection, and recheck capacity. The browser
regression holds a synthetic upload response, changes/restores the selection, then releases it.
Explicit fixture bytes make the intercepted upload equivalent in both browser engines. Visual
inspection found WebKit coordinate selects at 21px; an explicit height now matches the existing
control scale and is checked in the browser. No storage, contract or dependency changes.

V07b verification: six image-list browser cases pass across Chromium and WebKit, including
delayed import ownership and concurrent insertion. That race regression passed 40 repetitions.
Review approved the corrected implementation. Full matrix passed: 117 Rust tests (three ignored),
23 UI unit tests, two asset tests and 92 browser cases (four skipped), plus fmt, clippy, typecheck,
Storybook, reproducible assets and release build. Responsive screenshots were inspected; native
Safari was not rerun for this slice.

## V08 execution slices

V08a extends the existing readiness refusal boundary to definitive pre-run RPC errors. Named
refusals must validate; malformed refusal data, reuse of a claimed id and transport failures
remain uncertain. Save raw RPC evidence and an explicit zero-spend observation without inventing
a tool result. Explicit corrected attempts use new ids; independent wire tests also prove the
contract permits a released id to be reused. V08b owns reconciliation of existing claims and
known runs; V08c owns workspace messaging and browser retry checks. Each slice gets review and
the full matrix. Revert in reverse order; no stored-data migration or automatic replay.

V08a verification: the host regression first recorded `interrupted` for a plain RPC refusal.
After the fix, three refusal variants preserve zero spend and allow a fresh corrected attempt;
resubmitting the old HTTP identity returns its saved refusal without dispatch. Independent wire
tests prove released-id reuse and subsequent duplicate rejection. Negative unit cases retain
uncertainty for malformed/reused claims and known runs. Review found no blockers. Full matrix
passed: 120 Rust tests (three ignored), 23 UI unit tests, two asset tests, 92 browser cases
(four skipped), fmt, clippy, typecheck, Storybook, reproducible assets and release build.

V08b's known-run RPC regression initially labeled an allocated run as rejected: rmcp moves
notification metadata into extensions/context, so serializing only handler parameters lost it.
The receive-boundary regression also failed before the fix. Association now uses the complete
progress notification before the SDK schedules handlers or resolves a later response. Only
progress is serialized at this boundary, not large result payloads. Display progress restores
context metadata separately. RPC evidence is saved, reused references are schema- and identity-
checked, and associated runs use bounded resource recovery. Null reuse, queued cancellation and
disconnect remain uncertain. Independent wire coverage proves queue cancellation retains its
claim while a fresh id may run; host submissions never replay automatically.

Review found recovery lookup RPC errors could flow into no-run classification after cancellation.
A queued-cancel regression reproduced an incorrect rejected/zero-spend outcome when the runs
resource returned -32603. No-run classification now occurs only for the original action's RPC
response, before recovery. The regression waits for recovery's observable outcome, not merely
the earlier cancellation-request state. Control-read errors cannot release the original claim.

V08b final verification: review approved the provenance correction. All 19 execution tests,
seven independent wire tests, three readiness tests and the receive-boundary regression pass.
The full matrix passed: 125 Rust tests (three ignored), 23 UI unit tests, two asset tests,
92 browser cases (four skipped), fmt, clippy, typecheck, Storybook, asset reproducibility and
release build. The metadata correction adds no protocol or dependency change; no replay occurs.

V08c covers plain/structured refusal display, tool-provided reasons, explicit unchanged/corrected
retry identities and lost HTTP acknowledgements. The initial browser regressions reproduced
missing no-run messaging and reuse of an acknowledged attempt after a snapshot refresh failed.
Clear the cached submission immediately upon acknowledgement; keep it for uncertain transport
outcomes. Tests check the selected saved-attempt id before asserting a later result, so an old
completed result cannot satisfy the new execution check. No server API, storage or dependency
changes; reverting the workspace edit and its generated assets rolls back this UI slice.

V08c verification: review approved; all eight refusal/retry browser cases pass. The full matrix
passed: 125 Rust tests (three ignored), 23 UI unit tests, two asset tests and 100 browser cases
(four skipped), plus fmt, clippy, typecheck, Storybook, reproducible assets and release build.
The WebKit result screenshot was inspected: the refusal and correction appear without a false
missing-result warning. Native Safari was not rerun for this slice.

## V09 execution slices

V09a extracts preparation ownership from submission; V09b adds the retained resource lifecycle
and authenticated controls. Both receive review and the full matrix. Revert V09b before V09a;
no shared contract or library schema migration is required by the first slice.

The V09a tests first failed for unavailable preparation behavior, then independently reproduced
an abandoned submission when its requester disappeared during a database wait. An owned task now
continues through persistence and dispatch. Unit coverage checks exact paths/bytes after both
source and library bytes change, independent drafts, failed/duplicate cleanup, no early history,
and cleanup waiting until a transfer makes its old preview stale. Preparations hold a storage
read guard until durable ownership transfer; the worker cannot be dropped by its HTTP requester.

V09a verification: review approved without blockers. Three preparation lifecycle tests and all
20 execution tests pass. The full matrix passed: 129 Rust tests (three ignored), 23 UI unit tests,
two asset tests, 100 browser cases (four skipped), fmt, clippy, typecheck, Storybook, reproducible
assets and release build. The tampered-library fixture replaces its read-only file explicitly;
production asset immutability is unchanged. Retention and expiry remain V09b.

V09b retains preparations in the coordinator (16 entries, 1 GiB copied inputs, 15-minute absolute
expiry). Capacity is checked under the submission lock before copying; a weak-owner sweep and
lookup/consume pruning release expired copies. Duplicate requests do not renew expiry. Explicit
discard, restart, description invalidation and shutdown remove only unclaimed copies. The retained
storage guard requires cleanup to refuse before queuing its write lock; all host cleanup already
uses that coordinator entry point. Dispatch consumes a matching immutable request under discovery
synchronization and never falls back to rematerialization for a missing preparation. V09c owns
HTTP authorization and startup cleanup, so retained draft APIs remain internal in this slice.

Review caught a mismatched session key in the retained binding: session observations use `id`,
not run/result `session_id`. A regression changing only the actual session id failed before
the correction. Matching now requires a present, nonempty observed session id; a null/null pair
cannot pass. The independent unit fixture now uses the actual session-resource key.

V09b final verification: review approved the corrected session binding. Eight preparation tests
and 24 execution tests pass. The full matrix passed: 138 Rust tests (three ignored), 27 UI unit
tests, two asset tests and 100 browser cases (four opt-in skips), plus fmt, clippy, typecheck,
Storybook, reproducible assets and release build. Limits/expiry tests use disposable fixtures;
no real assets, paid providers, dependency pins or library schemas changed.

V09c exposes POST /api/preparations, GET/DELETE /api/preparations/{id}, and POST
/api/preparations/{id}/submit through the existing authenticated, same-origin workspace router.
Prepare and submit carry the strict Submission; the latter must match the path id and retained
request. There is no implicit submit or rematerialization on GET, prepare, discard or expiry.
Browser form/estimate integration remains V10–V11.

A versioned private ownership marker is written before input materialization. Startup validates
the execution root, skips every recorded attempt, and removes only private marked directories
whose entries are the marker and regular generated input files. Unmarked/unknown content and
symlinks are preserved. Cleanup never follows symlinks or recursively deletes discovered paths.
The attempt transaction transfers ownership before marker removal; a marker surviving that
transaction cannot make startup delete a saved attempt. Reverting this slice leaves markers and
unclaimed copies safely on disk; it does not require a library-schema downgrade.

The new API test first returned 405; startup tests first retained the orphan and accepted an
execution-root symlink. Targeted checks passed after implementation. Review approved and suggested
hard-link/public-marker coverage, now included. An additional unreadable-marker case reproduced
startup attempting to inspect a recorded attempt; recovery now checks the saved identity first,
without inspecting that directory's marker.

V09c final verification: review approved the saved-attempt exclusion ordering. Startup ownership,
authenticated controls and all execution/preparation targeted tests pass. Full matrix passed:
141 Rust tests (three ignored), 27 UI unit tests, two asset tests, 100 browser cases (four opt-in
skips), fmt, clippy, typecheck, Storybook, reproducible assets and release build. No public contract,
dependency, library schema or generated UI asset change.

## V10 controls

V10a keeps reserved estimate calls outside the persisted action gate. The staged-permit test
failed because estimate took the action permit and then failed its argument check. The actual
MCP client test independently failed with no dispatch authorization against the existing v0.3
control peer. Only exact describe/estimate names are now exempt. Tests verify untouched permit,
wire evidence, dispatch notification and progress association until the real action is sent;
similarly named arbitrary calls remain blocked. The independent peer returns a quote without a
run or action-call journal. No quote authorization or paid dispatch is introduced by this slice.

The first full matrix exposed a pre-existing cancellation assertion that assumed child exit and
worker cleanup completed within a 900 ms sleep. It now waits for observable tool availability
within the same four-second lifecycle bound as the adjacent child-exit test. The immediate
quit refusal and final quit success remain checked. Twenty consecutive runs passed under the
shared lock; review approved the transport changes and this test correction.

V10a's final full matrix passed: 143 Rust tests (three ignored), 27 UI unit tests, two asset
tests, 100 browser cases (four opt-in skips), formatting, clippy, typecheck, Storybook,
reproducible assets and release build. Lockfiles and generated UI assets are unchanged.
