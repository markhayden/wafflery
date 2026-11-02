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

### V10b slices and validation

V10b1 implements only validation; V10b2 owns bounded requests and preparation leases, and V10b3
connects the authenticated API and retained quotes. Each slice gets its own review/full matrix.
A timed-out request is not proof that the tool stopped reading. V10b2 must retain input ownership
until the reply completes or the child exits, keep those bytes/counts charged to retention limits,
and prevent late replies from resurrecting invalidated drafts.

V10b1's positive independent fixtures and structured-response/refusal tests failed before
implementation. The validator checks schema-valid contradictions too: action/tool/version/session
identity, nested ledger identity/arithmetic, readiness, cap equality, worst case versus cap/budget,
expired and over-24-hour quotes, and CLI quotes without serving sessions. A readiness refusal may
retain unknown price/configuration/cap; it remains refused. Numeric comparisons use 1e-6 USD;
conservative refusals within rounding tolerance remain refusals. No configuration digest or
binding is recreated by the host. The actual independent MCP transport probe also feeds its
response through the validator. The existing pinned time crate moves from dev-only to runtime
with parsing enabled; no version or lockfile change is needed. There is no attempt, library schema,
UI or dispatch change in this slice.

V10b1 passed review and the full locked matrix: 150 Rust tests (three ignored), 27 UI unit tests,
two asset tests, 100 browser cases (four opt-in skips), fmt, clippy, typecheck, Storybook,
reproducible UI assets and release build. Lockfiles and generated UI assets remain unchanged.

### V10b2 request ownership boundary

Preparations now have reader leases. Removing a draft's availability does not remove an active
reader's copies, free its capacity charge or let cleanup wait behind an invisible storage guard.
Dispatch cannot take paths while an estimate reads them. Late completion must match the same
prepared object, declaration and process session before a quote can be accepted.

The estimate deadline includes waiting for the ledger mutex. No request is sent if that wait
uses the deadline. Once sent, a timed-out RPC continues in an owned worker with its input lease
and per-tool estimate slot. Its late reply is discarded without reacquiring the ledger mutex;
transport loss instead retains ownership until the child has exited. The requester receives a
bounded timeout, with no automatic retry. Shutdown closes clients before joining these workers.

The lease tests failed before implementation, then passed for discard, expiry, restart,
declaration changes and shutdown invalidation. Live peer tests failed before the control service
existed and now cover exact copied inputs during a held free run, abandoned requesters,
per-tool estimate limits, the actual 30-second deadline, malformed replies, plain/named errors,
and graceful/forced shutdown. A disconnected peer deliberately survives EOF: files remain until
its owned child is confirmed stopped. The late-ledger test advances Tokio's test clock only after
the external peer's started marker, avoiding a scheduling-dependent short deadline. Tokio's
existing pinned version enables test-util only as a dev feature; no version/lockfile change.

V10b2 passed final review and the full shared-lock matrix: 160 Rust tests (three ignored),
27 UI unit tests, two asset tests, 100 browser cases (four opt-in skips), fmt, clippy, typecheck,
Storybook, reproducible UI assets and release build. No lockfile or generated UI asset changed.
Rollback this ownership slice as a unit; V10b1's standalone validation may remain.

### V10b3 authenticated estimates

Add strict empty-body POST and GET controls at `/api/preparations/{id}/estimate`. The browser
gets a preparation receipt and an explicit allowlist of estimate display fields, never the
binding, copied input paths or raw effective extensions. Opaque bindings stay with the exact
retained preparation for V12. GET does not renew preparation or quote lifetime. Both UTC expiry
and elapsed monotonic lifetime apply.

An accepted re-estimation clears the prior quote; rejected busy or malformed API requests do
not. Every accepted estimate owns a private ticket. A reported timeout invalidates only that
ticket, including a quote saved just before the timeout won; late cleanup cannot remove a newer
estimate. Publication checks the same prepared object, ticket, current declaration/session and
expiry under the retained-map lock. Rejected/expired preparations never regain availability.

Workspace snapshots show the newest validated session observation without changing the input
declaration revision. A separate short-lived cache lock preserves the latest observation while
an estimate holds the request mutex; it is never spending authorization. This slice leaves the
paid guard closed.

The storage/expiry tests failed before quote retention existed, and the authenticated POST test
failed with the absent route before API wiring. The snapshot regression saw the cached zero
balance instead of the peer's new observation. Tests now cover display allowlisting, no lifetime
renewal, ticket-specific timeout invalidation, refusal unknowns, unchanged draft revision and
zero attempts. A further independent-peer test found delivery flags and file locators included in
its new image-quote binding. The fake now binds image content and priced arguments/configuration,
excluding delivery flags; the host still treats the digest as opaque.

Review found that falling back to the original declaration while an estimate held the ledger
mutex could roll the display back from 0.08 to zero. The live sequence regression reproduced
that failure. Both validated publication paths now update a separate cache under serialized
ledger ownership, and snapshots read that cache instead. The late-response test also checks that
an expired estimate cannot overwrite the newer cached balance. The first full-matrix command was
cancelled while waiting for the shared lock, before it ran any checks, to fix this review finding.

The subsequent matrix caught a duplicate test-fixture module through Clippy. Both unit-test
consumers now import one test-only fixture module; runtime behavior is unchanged.

The browser matrix found a WebKit layout-test setup failure: no attempt was sent, and the
required label remained empty immediately after fill while the image import completed. No
parameter-reset path was found; the exact browser mechanism remains unconfirmed. Twenty
repetitions per engine with a value assertion passed before the setup change. The layout test
now waits for the imported preview and idle input, establishes label focus, and asserts the
filled value before checking scrolling. No timeout, retry or product behavior changed.

A second full browser pass reproduced the same empty-label setup in the missing-evidence test,
again without a dispatched attempt. These setup prerequisites now live in one sample-input
helper used by the existing immediate-upload/label scenarios. Import concurrency tests retain
their explicit controls. The repeated failure changes only test setup, not paid controls or
runtime behavior; the browser mechanism is still not claimed as established.

V10b3 passed final review and every full-matrix check: 167 Rust tests (three ignored),
27 UI unit tests, two asset tests, fmt, clippy, typecheck, Storybook, reproducible assets,
release build and 100 browser cases (four opt-in skips). After the setup-only correction,
80 stress cases and the complete browser suite passed; unchanged Rust/build checks were not
repeated. Lockfiles and generated UI assets are unchanged. The common browser setup correction
is a separate test commit before the estimate API commit. Revert the API/lifetime slice as a
unit; the test correction and V10b1/V10b2 may remain.

### Rascal explicit input directories

Rascal main `9ee6bc8` refused the old smoke descriptor's caller image path: the offline run
returned invalid and could not preserve input evidence. Adding the explicit execution-directory
grant made the real Rust smoke pass. The browser check then exposed a module boundary issue:
Playwright loads helpers outside the UI package as CommonJS. The shared argument builder now
lives beside the existing ESM offline-pipeline helper, imported by the development script.
It creates and canonicalizes only the owned execution directory before serving starts.

The real Rascal 0.1.1 / table-manners 0.3.0 smoke passed with the new grant, as did both
real-browser cases. The selected executable is a mutable development build, not an archived
release acceptance target; V18 remains open. Final review and the full shared-lock matrix passed:
167 Rust tests (three opt-in tests ignored), the additional real Rust smoke, 27 UI unit tests,
two asset tests, typecheck, fmt, clippy, Storybook, reproducible assets, release build and
102 browser cases (two Squint opt-in skips). No lockfile, UI asset or sibling file changed.

### V11a browser estimate controls

The browser client now has typed preparation, estimate, read and discard controls. Preparation
responses must match the submitted identity; quotes must match every receipt field and the
selected process session. Controls send exact request bodies once, without caller-supplied
quote fields, automatic retries or attempts. Shared readiness and session validation protect
display data. Unknown cost and uncapped allowance remain null. The positive cap parser accepts
finite positive decimal values without defaulting or clamping; the expiry helper uses the
shorter preparation/quote lifetime. Neither helper authorizes dispatch.

Five targeted tests were written before the client module existed and then passed, covering
invalid caps, unknown values, malformed/contradictory views, wrong preparation/session receipts,
exact HTTP bodies and zero automatic retries. Typecheck caught a widened fixture literal;
the independent fixture now declares its view type. The module is not wired into the workspace
until V11c; V11b supplies the component states first. The generic paid guard remains closed.

V11a passed review and the full shared-lock matrix: 167 Rust tests (three ignored), 32 UI unit
tests, two asset tests, 100 browser cases (four opt-in skips), fmt, clippy, typecheck, Storybook,
reproducible UI assets and release build. Only the checked-in UI input manifest changed;
compiled UI output bytes and dependency lockfiles are unchanged.

### V11b cost presentation

The controlled cost component keeps the user's cap text intact, accepts only positive finite
values for an explicit estimate request and respects the surrounding form's validity. It
shows worst case, effective model, cap, current session allowance and unresolved spend, with
configuration/session/expiry details expandable. Readiness, cap and budget refusals have distinct
copy; an unavailable backend is not described as merely needing setup. A changed session hides
the old quote; a ledger-only update keeps it visible with a warning if the balance has fallen.
Workspace lifecycle wiring remains V11c, and paid dispatch remains gated.

Browser tests were written before the component existed. The initial six Chromium/WebKit cases
passed after implementation, including keyboard-only estimation, invalid cap text, refusal and
loading/error/stale states, unknown versus uncapped amounts, and narrow/wide layouts. Screenshots
at 320 and 1440 pixels were inspected; details wrap without horizontal overflow. A further test
proved unavailable-backend copy needed to differ from missing setup. The shared USD formatter's
new regression first displayed a positive sub-microdollar amount as zero; it now displays
`< $0.000001`, preserving the distinction from free work. Component/stories/browser coverage and
the small shared formatter regression form one presentation slice.

V11b passed final review, all eight targeted Chromium/WebKit component cases and the full
shared-lock matrix: 167 Rust tests (three ignored), 32 UI unit tests, two asset tests,
108 browser cases (four opt-in skips), fmt, clippy, typecheck, Storybook, reproducible assets
and release build. Generated UI assets were refreshed for the shared formatter; lockfiles
are unchanged. This slice can be reverted independently until the workspace consumes it.


### V11c live draft estimates

The workspace owns a preparation only for the exact selected tool, action, declaration, session
and draft. Explicit estimation copies the current ordered image bindings and coordinates and
normalizes the user's positive cap to a number. Edits discard that preparation, hide its quote
and never trigger another estimate. Late preparation or estimate replies are discarded; expiry
uses the shorter preparation/quote deadline and checks visibility restoration. Disposal is best
effort after disconnection, with the host's existing TTL bounding abandoned copies.

The paid cap has one editor, including for whole-object JSON forms. JSON parameters cannot
replace it. Review found that malformed JSON initially retained a quote for the last valid
parameters; a failing browser regression demonstrated it. A separate input-edit revision now
invalidates ownership even when the editor cannot parse the new text, without remounting the
editor or losing its last valid parameters. Correcting the text still requires an explicit
estimate. The paid submit guard stays closed on both client and server until V12.

Live Chromium/WebKit checks cover exact arguments, no attempts/history, cap and parameter edits,
late preparation and estimate replies, expiry, restart, description changes, ordered image
coordinates, configured-cap refusal without clamping, unknown cost, and a second tab reducing
session allowance while the first tab retains its quote. Additional form coverage proves that
whole-object JSON cannot overwrite the dedicated cap. The independent fake supplies these
states without network or provider calls; its ledger mutations are synthetic. This cohesive
lifecycle slice exceeds the usual authored-file guideline to keep fake scenarios, UI wiring,
stories, tests and their documentation together.

V11c passed final review and all 28 targeted lifecycle/form browser cases. The full shared-lock
matrix passed: 167 Rust tests (three ignored), 33 UI unit tests, two asset tests, 132 browser
cases (four opt-in skips), fmt, clippy, typecheck, Storybook, reproducible assets and release
build. Live cost screenshots were inspected in Chromium and WebKit. Generated UI assets are
updated; dependency lockfiles are unchanged. Revert this lifecycle slice before V11b/V11a when
rolling back; V12 remains gated.


### V12a dispatch estimate boundary

The transport permit now retains a validated quote when pricing is required. Before recording or
sending an action, it checks the exact opaque binding, allowed decision, matching action and
quote lifetime. Unpriced permits reject injected estimate metadata. The final expiry check runs
after queued database persistence and before polling the wire write. If the quote expired during
persistence, the recorded wire intent remains inspectable but the sent signal stays false.
The coordinator still supplies only unpriced permits and refuses paid actions; consumption and
paid activation remain V12b/c.

A regression first proved an unpriced permit could send injected estimate metadata. All nine
transport checks now pass, including exact metadata preservation, missing/substituted/null
bindings, refused/expired quotes, mismatched actions, unchanged control requests and an explicitly
blocked database write across a monotonic clock advance. No test invokes a provider.

V12a passed final review and the full shared-lock matrix: 170 Rust tests (three ignored),
33 UI unit tests, two asset tests, 132 browser cases (four opt-in skips), fmt, clippy, typecheck,
Storybook, reproducible assets and release build. UI assets and dependency lockfiles are unchanged.
This internal transport boundary can be reverted independently until V12b/c consume it.


### V12b owned quote consumption

Preparation consumption now checks request equality, process/tool session, declaration, reader
ownership and paid quote eligibility under one lock before removing the draft. A paid draft must
have its own non-invalidated estimate ticket and an allowed unexpired quote with matching tool,
action, session and positive cap. The consumed owner and quote transfer together; neither expiry
cleanup nor another submission can take them again. The coordinator's paid guard remains closed
until V12c.

Paid persistence rechecks the exact copied inputs in a blocking worker using bounded reads,
size and SHA-256 checks, regular owned files, no symlinks or hard links, and the owned directory.
Every list occurrence is verified. Review caught an accepted empty optional scalar slot being
mistaken for a missing file; its failing regression now also exercises an altered later list
copy. The verifier skips only explicitly empty supplied slots and checks nonempty list counts.
The tool remains authoritative for binding verification when the call arrives.

The attempt transaction now saves selected authorization facts in `observations.estimate`
alongside input references and the immutable request. Binding, cap, worst case, effective model/
config identity, expiry and attribution are retained; arbitrary effective/debug extensions are
excluded. Source metadata, retained ownership, quote validation, file verification and the atomic
storage path form one coherent boundary across more than the usual five authored files.

The missing-quote and optional-image regressions failed before their fixes. V12b passed all 17
preparation checks, final review and the full shared-lock matrix: 175 Rust tests (three ignored),
33 UI unit tests, two asset tests, 132 browser cases (four opt-in skips), fmt, clippy, typecheck,
Storybook, reproducible assets and release build. UI assets and dependency lockfiles are unchanged.
Rollback this consumption slice before V12a; ordinary submissions still cannot authorize paid work.


### V12c bound paid submission

The authenticated prepared-call path now enables paid execution after atomic owned-quote
consumption and input verification. It passes the exact stored binding in MCP metadata alongside
client-run correlation and the SDK progress token. The transport permit holds the same validated
quote. Ordinary submissions still cannot authorize new paid work. Duplicate acknowledged
submissions return the saved attempt, while a new attempt needs fresh preparation and estimation.
Browser paid Run remains disabled until V12d.

The independent peer recomputes bindings, verifies expiry/cap/session budget and records a
synthetic $0.08 spend against a $0.12 worst case, without provider calls. Its run and journal use
the normal contract evidence path. Selected quote facts are present in the exported archive.
A failing regression first reproduced the closed paid guard; the initial three integration cases
then passed exact wire/evidence, idempotence/export, missing/refused/changed/expired/session/declaration
cases and authenticated strict API access. Concurrent duplicate coverage and final fake refinements
are included in the full matrix. Final review passed. The shared-lock matrix passed 178 Rust
tests (three ignored), 33 UI unit tests, two asset tests, 132 browser cases (four opt-in skips),
fmt, clippy, typecheck, Storybook, reproducible assets and release build. No dependency or UI
asset changes. Rollback this slice before V12b; browser Run remains closed until V12d.


### V12d reviewed browser Run

Paid Run now requires the current allowed, unexpired owned estimate. It submits the immutable
prepared request, never reconstructed form values or a caller-supplied binding. Another paid
attempt requires a fresh explicit estimate. Edits, invalid JSON, image order/coordinates,
declaration/session changes and expiry keep Run disabled until the new draft is reviewed.

Submission takes ownership before the request leaves the browser. Later edits and quote expiry
cannot delete its inputs. Uncertain acknowledgement retains the original request for explicit
same-ID Retry submission; acknowledged IDs are never reused for a new attempt. Review found two
edge cases: mismatched acknowledgement must not clear ownership, and retry must work without a
current tool declaration. Their regressions failed first and passed after correcting both paths.
Structural response checks preserve ordered image arrays and tolerate JSON object-key ordering.
The setup copy now describes the available estimate/Run flow.

Targeted Chromium/WebKit checks passed 54 cases, including acknowledgement loss, wrong response,
declaration loss and estimate invalidation. The cost panel was visually checked in WebKit.
Two additional browser cases cover held submissions during edits and definite refusal recovery;
the full shared-lock matrix passed: 178 Rust tests (three ignored), 35 UI unit tests, two asset
tests, 144 browser cases (four opt-in skips), fmt, clippy, typecheck, Storybook, reproducible
assets and release build. Final review passed. The API adapter, estimate owner and workspace form are kept
in one slice with their tests despite exceeding the usual five authored files. Rollback V12d
before V12c; no server wire or storage change is introduced by this browser slice.

### V13a paid outcomes and readiness refresh

The independent paid peer now exercises zero-spend stale/cap/credential failures, known failure
charges, unresolved provider outcomes, sequential budget changes, readiness validation, and
cancellation after a synthetic request was sent. Held calls reserve their worst case atomically
and release the mutex while waiting; describe, estimate and session reads keep responding.
Cancellation settles unresolved worst-case spend instead of clearing it. Cancellation before
allocation keeps its client claim and creates no run; only a definitive no-run RPC refusal releases it.

The host refreshes discovery after a credential error or no-run setup refusal, before releasing
execution availability. It does not retry or restart. A regression with that refresh removed
showed the obsolete ready state after the provider rejected authentication. Restoring it updated
the declaration to needs_setup/invalid. All eight paid integration tests now pass, including
separate sequential-budget and present-to-valid cases, no-run setup refusal, and cancelled spend.
The initial full matrix passed. Review then caught a cancelled client claim being released and
missing held-call heartbeats. Both regressions failed before correction; the extended free/paid
wire reuse check and all eight paid tests now pass. Held calls emit progress each second with the
original token and run metadata. Final review passed. The post-correction shared-lock matrix passed 183 Rust tests (three ignored),
35 UI unit tests, two asset tests, 144 browser cases (four opt-in skips), fmt, clippy, typecheck,
Storybook, reproducible assets and release build. No lockfile or UI asset changes.
This slice groups the fake's protocol,
ledger and execution paths with the host observer and outcome tests; the renderer stays V13b.
Rollback the host refresh and scripted peer scenarios together before reverting V12.


### V13b selected outcome guidance

The result panel now explains stale estimates, provider authentication rejection, caps and
session budget failures. No-run setup refusals remain distinct from allocated error results.
Known failure/cancellation charges are retained; unresolved spend is displayed as “at most,”
with upward rounding so display precision cannot understate the bound. Missing or invalid spend
never becomes a fabricated zero. Other outputs and the inspection/export drawers remain available.

Review tool setup binds the drawer to the saved request's tool and action, independently of an
edited current draft. Refresh and deliberate restart follow that same context. Guidance does
not retry, increase a cap or reset a budget. New stories cover successful, failed, cancelled,
no-run and unknown outcomes; narrow viewport checks cover wrapping and keyboard-accessible setup.

Six unit tests pass. Targeted Chromium/WebKit checks passed 54 cases, including live scripted
errors, a later budget refusal, saved-action setup after an edited draft, and existing submission/
setup regressions. The unresolved-spend result was visually inspected in WebKit. Review passed;
The first full browser pass exposed a duplicated live cancellation status; only no-run summaries
now announce an additional status, while allocated runs retain the canonical header status.
A failing regression also corrected free-action guidance that requested an unavailable estimate.
Guidance uses the saved action's cost declaration, never the edited draft. Six targeted browser
regressions passed after correction. Final review and the shared-lock full matrix passed:
183 Rust tests (three ignored), 41 UI unit tests, two asset tests, 162 Chromium/WebKit cases
(four opt-in skips), fmt, clippy, typecheck, Storybook, reproducible assets and release build.
Component/model/style/story and workspace wiring are one
cohesive presentation slice with their tests. Rollback this UI slice before V13a; stored evidence
and server behavior are unchanged.


### V13c paid workflow checkpoint

The [V-B record](v03-paid-checkpoint.md) covers cancellation, saved history, exported exact
binding/cap/session evidence, and host restart without rewriting old unresolved spend. The new
journey passed in both browsers. Full-matrix verification found an existing offscreen label
edit that never took effect before retry; the shared helper now explicitly focuses and verifies
edits. All 120 acknowledgement repetitions passed, followed by the complete shared-lock matrix
(183 Rust tests, three ignored; 41 UI units; two asset tests; 164 browser cases, four skipped).
Review passed. No production code or assets changed. V14 is split into command resolution and
process ownership so the next execution boundary can be reviewed before it is connected.

### V14a saved render command resolution

The resolver selects the render from saved evidence and builds literal argv for the serving
client's canonical executable. It checks action/tool attribution, registered run-directory
access, whole-argument substitution, the exact host origin, timeout shape and command bounds.
An open directory handle permits a device/inode recheck before launch. A focused red regression
caught re-canonicalization silently following changed registration symlinks; it now refuses
those redirects at both resolution and command construction. No child, route or frame is opened.

Six focused Rust checks cover server/document forms, spaces and shell metacharacters, malformed
commands/origins, unavailable/escaped directories, explicit authorized copies and redirected
registration paths. Review and the shared-lock full matrix passed: 189 Rust tests (three ignored),
41 UI unit tests, two asset tests, 164 Chromium/WebKit cases (four opt-in skips), fmt, clippy,
typecheck, Storybook, reproducible assets and release build. This slice adds no dependencies, library
migration or UI assets. Roll it back after dependent process/viewer slices.

### V14b1 owned render worker

A dedicated worker retains each spawned child and drains bounded stdout/stderr concurrently.
Graceful close is stdin EOF; a stuck process remains observable for an explicit force-stop.
Control acknowledgement does not depend on observing a transient watch state, and child-wait
errors retain ownership rather than claiming exit. The synthetic renderer exercises literal
arguments, the minimal environment, untouched run files, EOF/drop closure, noisy diagnostics,
early error exit and forced stop after the grace period. Initial missing-worker tests failed;
all four process tests then passed. Clippy caught duplicate fixture imports, now shared at the
render module boundary. Review and the shared-lock full matrix passed: 193 Rust tests (three
ignored), 41 UI unit tests, two asset tests, 164 browser cases (four opt-in skips), fmt, clippy,
typecheck, Storybook, reproducible assets and release build. Coordinator retention,
process groups, ready URLs and report content are still subsequent gates. This cohesive slice
keeps the worker, its real child fixture, module wiring, shared fixture import and tests together
despite exceeding the usual five authored files; it introduces no browser or public host API.

### V14b2 coordinator-held report controls

The coordinator now keeps view handles across HTTP response loss, enforces one live view per
saved attempt and 16 retained slots, and uses the current serving client's canonical executable
only when its tool name matches the saved declaration. Strict authenticated controls accept a
view name, not argv or a caller-selected origin. Graceful/force shutdown includes every retained
view. Responses expose process state and bounded diagnostics; raw stdout and report content
remain private until the later output/framing gates.

New API integration tests first failed on missing routes. They cover private/strict request
boundaries, concurrent idempotent opens, replacement only after exit, startup failure without
run mutation, missing directories, stuck-view shutdown ownership and capacity reclamation.
All three targeted tests passed. Review and the shared-lock full matrix passed: 196 Rust tests
(three ignored), 41 UI unit tests, two asset tests, 164 browser cases (four opt-in skips), fmt,
clippy, typecheck, Storybook, reproducible assets and release build. Coordinator, shutdown, host routing and
the independent fixture tests stay in one slice because exposing the controls before retaining
shutdown ownership would be unsafe. This exceeds the usual five authored files; no UI assets,
library migration or dependency changes are introduced.

### V15a server readiness validation

The readiness validator accepts one complete literal-loopback HTTP URL line on a separate
origin, validates process/output health throughout viewing, and makes startup failure sticky.
It checks the raw authority before URL normalization and bounds startup deadlines to 1–60
whole seconds. Supervision and real HTTP report fixtures remain V15b; no report content is
exposed yet.

Five new tests first failed because the validator did not exist, then passed. Review found no
required fixes. The shared-lock full matrix passed: 201 Rust tests (three ignored), 41 UI unit
tests, two asset tests, 164 browser cases (four opt-in skips), fmt, clippy, typecheck, Storybook,
reproducible assets and release build.

### V15b retained server supervision

Retained server views now own a readiness worker that wakes on process/output changes or the
original startup deadline. Failure closes the renderer's stdin without requiring an observing
browser. Explicit close intent survives coalesced process notifications, so normal close does
not become a startup error. Addresses remain private and no tool HTML is embedded.

The independent fixture runs a real loopback HTTP server with the supplied exact frame
ancestor. Two new integration tests first failed on missing readiness and timeout transitions,
then passed alongside the three existing report-control tests. They verify delayed startup,
HTTP content and framing policy, listener shutdown after extra stdout without polling the
host, timeout, malformed URL, early exit and unchanged saved evidence. An early exit can be
observed as stdout EOF before the exit status; the test requires the failure and exit code
without prescribing event order. Review found no required fixes. This cohesive slice exceeds
the usual five-file budget because it connects process close intent, supervision, coordinator
observations and the independent HTTP fixture together. The shared-lock full matrix passed:
203 Rust tests (three ignored), 41 UI unit tests, two asset tests, 164 browser cases (four opt-in
skips), fmt, clippy, typecheck, Storybook, reproducible assets and release build.

### V15c1 owned report process groups

The new macOS group owner retains its leader unreaped while observing exit through kqueue
and checking bounded membership output from the fixed system `ps` executable. Unknown,
malformed, timed-out or truncated observations preserve ownership. Stopped members count as
live; only an observed all-zombie group can release its leader identity. Later signals are
rejected. Helper observation is owned by an independent bounded worker, including cancellation
of its caller. The existing pinned nix dependency enables its event and signal features.

Four new tests first failed on the missing owner, then passed: strict observations, owned
SIGTERM/reap, deterministic exit before event registration, and observer failure/overflow/
timeout. Review found no blockers; live descendant integration remains V15c2. The shared-lock
full matrix passed: 207 Rust tests (three ignored), 41 UI unit tests, two asset tests, 164 browser
cases (four opt-in skips), fmt, clippy, typecheck, Storybook, reproducible assets and release build.
The current report worker remains unchanged until V15c2. Module wiring, dependency features,
owner, tests and design notes stay together despite exceeding the usual five-file budget.

### V15c2 group lifetime integration

Report workers now use the retained group owner. Close gives EOF two seconds, then sends
SIGTERM to the same owned group and waits another two seconds. Only explicit force sends
SIGKILL. Early leader exit with a surviving descendant starts graceful group cleanup and
keeps the view slot until all members stop. Membership failures retain ownership. The worker
completes each bounded observer call before it can finish, and report launch refuses unsupported
platforms before spawning.

Two new process tests failed against the previous worker, then all six passed. They cover
EOF-ignoring/clean-SIGTERM exit, graceful descendant termination, and an early leader exit with
a SIGTERM-ignoring descendant that requires explicit force. The tests verify the descendant
stopped and the saved directory stayed unchanged. Synthetic orphan fixtures expire after 30
seconds as a red-test cleanup guard; assertions finish well before that deadline. Review and
the shared-lock full matrix passed: 209 Rust tests (three ignored), 41 UI unit tests, two asset
tests, 164 browser cases (four opt-in skips), fmt, clippy, typecheck, Storybook, reproducible
assets and release build. The lifecycle worker, group stdio transfer, independent descendant
fixture, tests and docs form one cohesive slice beyond the usual five-file budget.

### V15d idle viewer expiry

Every view now has an in-memory five-minute idle deadline. Explicit activity or reopening the
same active view renews it; status polling does not. Renewal and expiry are atomic, and late
activity cannot revive a closed/expired view. Expiry requests the same graceful owned-group
close and does not force-kill. An `idle_expired` observation explains closure without changing
the saved run. The browser's visible-inspection activity wiring remains V17b.

Three new idle tests and an activity API test first failed, then passed. They cover deadline
renewal, old-timer races, late rejection, autonomous cleanup of a real abandoned renderer,
strict authentication/body validation and closed/replaced IDs. Review found no blockers.
The shared-lock full matrix passed: 213 Rust tests (three ignored), 41 UI unit tests, two asset
tests, 164 browser cases (four opt-in skips), fmt, clippy, typecheck, Storybook, reproducible
assets and release build. The lease, coordinator/API wiring, shared process fixture access and
tests remain together beyond the usual file budget. V15 is complete; content isolation and
visible report controls remain V16/V17.

### V16a document output grants

A successful document renderer grants access to exactly its returned regular file outside
the pinned run directory. The host reads an immutable copy of at most 32 MiB with a five-second
deadline. Ancestor identity checks reject in-run aliases; before/open/after snapshots reject
replacement and mutation. No adjacent file is read and the original is left untouched. Six
tests cover output/status validation, missing/non-regular/oversized files, aliases, run movement,
replacement races and retained reader capacity after timeout. Review strengthened ancestor
identity validation and global reader permits, then approved. The sixteen permits remain held
inside blocking workers until completion, even if their caller times out or disconnects.

The first matrix exposed an existing offscreen WebKit edit in the held-preparation test.
The [test correction](../reviews/2026-11-02-preparation-edit.md) uses the existing focused-edit
helper and observes invalidation before releasing responses. All 160 focused browser repetitions
passed with zero retries; review approved. The final shared-lock full matrix passed: 221 Rust
tests (three ignored), 41 UI unit tests, two asset tests, 164 browser cases (four opt-in skips),
fmt, clippy, typecheck, Storybook, reproducible assets and release build. The reader/module,
tests and matching plan/engineering/execution docs form a cohesive slice beyond the usual
five-file budget. V16b supplies isolated serving; V16c connects the retained view lifecycle.

### V16b isolated document serving

The document server owns a separate literal-loopback port and private path, serves immutable
bytes without a filesystem router, validates Host exactly and grants no CORS access. CSP
restricts resource loads to inline/embedded content, sandboxes active documents and permits
only the workspace origin as a frame ancestor. Bounded violation reports mark the content
unavailable without retaining URLs/snippets or modifying the run. Browser enforcement and
nested framing remain V17's integration checks; these tests verify the HTTP boundary.

Each owner tracks at most eight connections with bounded headers/body and a ten-second total
connection deadline. Close and last-owner drop abort and join every connection before reporting
closure. Hyper and hyper-util become direct dependencies at existing locked versions to provide
explicit connection ownership; no package version changed. Four tests failed before the server
existed, then passed, covering exact content, neighboring paths, Host aliases/duplicates, private
headers, malformed/oversized violations, framing origins and stalled connections on close/drop.

Review approved. The shared-lock full matrix passed: 225 Rust tests (three ignored), 41 UI unit
tests, two asset tests, 164 browser cases (four skips), fmt, clippy, typecheck, Storybook,
reproducible assets and release build. Dependencies, server/module, tests and matching docs form
one cohesive slice beyond the usual file budget. V16c1 adds retained generation/read/content
supervision; V16c2 connects it to coordinator capacity, activity and shutdown.
