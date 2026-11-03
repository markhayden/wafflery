# Independent offline peer

`sample-lab` is Wafflery's owned test tool. It is explicitly illustrative; it does not stand
in for Rascal's repair operations or Squint's judgments. Its wire implementation does not use
rmcp or Wafflery's coordinator/client. It may use the published contract's data/schema crate.

The fixtures target published `table-manners =0.4.0`. `fixtures.rs` builds deterministic
description/result documents. The owned PNG is `ui/tests/fixtures/sample.png`, a generated RGB
gradient; its byte fingerprint is computed, not copied from another tool's output. The fixture
input identity is a documented placeholder until the wire peer computes actual request identity.
The completed schema fixture deliberately has no outputs; wire tests will verify actual image
outputs. Unsupported-version declarations are negative scenarios, including the former 0.1.0 version.

Cancellation follows CONTRACT.md sections 2.1 and 4.2: status cancelled, null error, spend to date,
and no MCP result for a cancelled request. The shape was cross-checked against
`table-manners/fixtures/squint/run-cancelled.json` at `8e4d0c3`; it is not a Rascal fixture.
Schema retrieval follows `table-manners/tests/fixtures.rs`, resolves embedded URNs only, and
never retrieves arbitrary network references.

The host's production version/dispatch guard is implemented with its client in F10/F11; these
fixture tests do not substitute for that execution boundary.

## Run the wire checks

`cargo test --locked --test fake_wire` builds the `sample-lab` example, then starts it in
temporary directories using an independently written line-delimited client. The probe checks
2025-11-25 initialization, explicit input grants, tool pagination, both path and inline image calls, expected
fingerprint refusal, output digests, multiple run resources, cancellation and EOF shutdown.

The example takes `<workspace> [scenario]`. `normal` and `slow` execute the offline image
operation. `held` emits running progress and waits until the test writes `<workspace>/release-run`.
It consumes that file before completing, sends heartbeats while held, and cancellation/EOF
still stop the wait. Browser tab
handoff tests release it only after observing progress and closing the launching tab, so the
scenario has no timed automatic completion or dependency on sleep scheduling. The signal belongs
only to this disposable test peer, not the host or shared contract. `paid`, `unsupported-contract`, `wrong-protocol` and `malformed` are negative test
scenarios, not full conformance claims; none makes a paid or remote call. Only the host's owned
test descriptor will opt into this executable. No real tool is discovered or installed by it.

## Optional real Rascal smoke

Select a local binary explicitly (never discovered or installed automatically). For Rust real-peer
probes, first create isolated lookup/cache directories and keep these variables set for the probe:

```sh
probe_root=$(mktemp -d)
mkdir -p "$probe_root/config" "$probe_root/cache" "$probe_root/data"
export XDG_CONFIG_HOME="$probe_root/config"
export XDG_CACHE_HOME="$probe_root/cache"
export XDG_DATA_HOME="$probe_root/data"
```

Then select the approved contract 0.4.0 binary:

```sh
WAFFLERY_RASCAL_BIN=/absolute/path/to/rascal cargo test --locked --test rascal_smoke -- --ignored --nocapture
```

This uses the same host coordinator, fresh no-paid check, exact contract-version allowlist,
image fingerprints and evidence capture as the fake. The child uses the host
environment allowlist and isolated XDG directories, receives no selected config or filler, and
gets temporary workspace/run directories. It checks a real
`seam` result, zero spend, verified outputs/journal and unchanged original bytes. `mask_rect`
requires a location binding, which is outside the foundation's image-and-parameters form.

The seam needs explicit equirectangular/wrap image coordinates. The host's optional `frames`
field records that shared image-reference metadata alongside ordered asset bindings. Ordinary
images default to flat/bounded. This metadata is validated using the published image-reference
schema; the host never interprets a tool's private projection parameter. The smoke explicitly
sets Rascal's declared parameter as well. A flat seam can complete without producing an image,
so that alone is not a successful output smoke.

Explicitly running an ignored probe fails if its binary, isolated directories or contract version
are invalid. Without `--ignored`, Cargo reports it ignored. Both declared report kinds must serve
HTML with the exact frame-ancestor policy, close gracefully, and leave saved evidence unchanged.
Do not loosen the version, spend or fingerprint guards to pass these checks.

## Optional real Rascal browser journey

```sh
WAFFLERY_RASCAL_BIN=/absolute/path/to/rascal bun run --cwd ui test:browser
```

The optional Chromium/WebKit journey uses explicit registration and the same generic UI as any
tool. It checks a real seam mask, saved inputs, recorded call, image evidence download, independent
example loading, and history/output preservation across a host restart. Without the environment
variable those two cases are reported skipped. Choose a stable contract 0.4.0 executable; rebuilding
a selected binary against a newer draft during the suite will correctly make it unavailable.

`ui/tests/fixtures/panorama.png` is an owned, procedurally generated 2048×1024 RGBA gradient:
`R = 60 + floor(80*x/2048)`, `G = 80 + floor(100*y/1024)`, `B = 110`, `A = 255`.
Its unequal left/right edges create a deterministic wrap seam. The browser checks actual mask
pixels contain both protected black and paint values above 127. The small 160×100 fake-tool
fixture rounds Rascal's default band below a pixel, so an image-presence assertion alone would
miss an empty mask. The panorama uses default recipe parameters. Source and cut-space mask have
different coordinates; the test requires side-by-side viewing and a disabled wipe, not invented
alignment. With no filler this verifies mask preparation, not a repaired image or model quality.

## Optional Squint and two-tool integration

Use an explicitly selected, stable contract 0.4.0 Squint binary built with `--features testkit`
at coordinator-approved revision `6dd1e3a` (M10). The coordinator
probe owns config/script/files, uses the host environment allowlist with the isolated XDG directories above, and supplies no credentials:

```sh
env -u ANTHROPIC_API_KEY WAFFLERY_SQUINT_BIN=/absolute/path/to/testkit/squint cargo test --locked --test squint_smoke -- --ignored --nocapture
```

This verifies discovery, truthful free metadata, a reference-consistency scripted result, zero spend,
recorded call, complete evidence and unchanged original images. Ordered references include a repeated
asset with distinct execution paths; both document and server reports are opened and closed. The
verdict is fixture data, not a model judgment. Other use cases are not certified by this probe.

With both `WAFFLERY_RASCAL_BIN` and `WAFFLERY_SQUINT_BIN` set to explicit stable binaries,
`bun run --cwd ui test:browser` includes the two-tool journey: Squint check, inspect/export,
example reload without execution, switch to Rascal and run a seam, restart, then restore Squint
history without mixing results. Without both variables those cases are skipped.

`real-peer-reports.spec.ts` additionally opens both tools' server/document renderers inside the
shared drawer after draft edits, observes their closure, and reopens the saved document after
host restart. Its Squint fixture uses ordered/repeated references. Browser fixtures isolate XDG
config/data/cache directories per host. See [V18 coverage and limits](../docs/plans/v03-real-peer-checkpoint.md).

The setup probe additionally verifies default `./squint.toml` lookup and explicit selected paths,
missing configuration/credentials, configuration frozen until restart, and real-profile catalog
estimates that create neither a host attempt nor a tool run. It requires `ANTHROPIC_API_KEY` unset
and makes no provider call. Rascal's second probe requires a `testkit` build: its local filler
reports a synthetic $0.04 cost to exercise bound dispatch, per-call caps, session budget refusal,
configuration identity and explicit restart with a fresh ledger. No real charge occurs.

The old estimate-as-run probe is retired: in v0.3, estimates are controls and never attempts.
Its v0.2 journal evidence remains [historical](../docs/handoffs/squint-estimate-journal.md). [Original discovery blockers](../docs/handoffs/squint-discovery-blockers.md)
are resolved. `scripts/dev-pipeline.ts` starts the same isolated offline setup through Squint's H1 launcher.

## v0.2 association and cancellation scenarios

The peer publishes a fresh session UUID per process and echoes client correlation IDs in progress,
run resources and results. The host captures the SDK-assigned progress token at the wire boundary.
`slow` exercises cancellation with progress association; `no-progress` and `paged` exercise
runs-list recovery. `unassociated` deliberately withholds evidence. `duplicate-pages`,
`conflict-once` and `endless-pages` verify that contradictions or incomplete searches keep the
outcome unconfirmed and the reservation held. These negative peers deliberately violate the
contract and are not conformance examples. `session-reset` proves a new ledger cannot silently
authorize dispatch. Paid actions require an owned priced preparation before dispatch.

## Control preparation peer (V01, migrated in M5.03)

`control_fixtures.rs` owns descriptions for `sample-lab` and `test-lens`, a ready offline action beside
an independently configured paid action, and session/estimate/refusal examples. These fixtures
perform no provider calls. Paid host dispatch additionally requires an owned priced preparation.
The preparation tests validate independently of host-generated types.

V02 replaces the temporary V01 schema archive with the exact contract dependency's embedded
schemas. Fixed timestamps/UUIDs in fixture examples are test data; serving processes provide
fresh sessions and live expiry. M5.03 uses the published =0.4.0 schemas with no path pin.

The `control-ready`, `control-lens`, `control-needs-setup`, `control-unavailable` and `control-budget` scenarios expose
those objects over MCP 2025-11-25. They are **control-only preparation probes**, not complete
execution peers: describe, estimate, session/runs reads and discovery work; action execution is
explicitly unsupported until its owning task. The normal/held execution scenarios also use v0.4.
`price_test` estimates are deterministic and local, with a scripted $0.12 worst case and $1
configured cap. Delivery flags do not affect the binding. Each process gets a fresh session;
no estimate allocates a run, claims a client run ID, reserves money or changes the ledger.
The wire tests validate returned objects against the dependency’s embedded schemas, compare the ledger
before and after controls/refusals, and verify the workspace remains empty.

V02 discovery negatives (`control-legacy`, `control-no-estimate`, `control-no-cap`) intentionally violate
one live boundary. The older `paid`/`becomes-paid` scenarios remain dispatch-guard probes:
they advertise the reserved estimate control but do not implement a paid execution flow.
Only the control-only scenarios currently implement estimate responses.

## Readiness scenarios

`readiness` and `readiness-held` describe an offline action with an optional missing filler,
beside a paid action whose ordinary unprepared submission the host blocks. `credential-state` under the peer
root selects present/valid/invalid/missing/unknown on the next description, without reading any
credential. Only requesting the optional filler produces -32602 not_ready, before claiming the
client ID or allocating a run; an omitted filler remains usable. Wire coverage reuses that
released ID successfully. The held variant uses the existing release-run gate for concurrent
refresh tests. The paid action is discovery-only; estimate execution remains in the control-only peer.

V03a validation: five wire tests and fake-peer clippy passed. Review found and corrected an
image-free action schema that incorrectly required source; input schemas now derive image fields
from declared slots. Both optional setup and that schema regression failed before their fixes.

## Configuration discovery scenarios

The `configuration` scenarios read a synthetic tool config from `--config`, XDG_CONFIG_HOME or
HOME. The independent peer reports config-dependent credential and non-secret env requirements.
`configuration-fails` rejects discovery; the synthetic config’s `extra_when_configured` flag
adds a requirement only in the serving process. Tests verify missing credentials remain setup requirements without a retry loop.
The fixture's startup journal records process/session boundaries and present variable names only,
never values. Tests confirm probe exit precedes serving startup, only the serving session is
published, and neither process executes an action. The existing `ignore-eof` scenario verifies
that an unresponsive probe stays owned and requires deliberate force authorization.

The `initialize-held` scenario writes an `initializing` marker and waits for
`release-initialize` before answering initialize. It exercises abandoning a restart requester
while the host still owns replacement startup; it makes no provider calls.

## List image evidence

`image-list` declares an optional `many` references slot; `image-list-required` makes it required.
The peer reads, fingerprints and records each image in order, including duplicates. The
`image-list-partial-{error,invalid,cancelled}` variants report source plus one reference;
`image-list-empty-{error,invalid,cancelled}` reports no input images. These are synthetic terminal
results, not provider calls. Host tests verify completed lists and preservation notices without
relabeling partial non-completed outcomes. Binding tests separately exercise per-occurrence
coordinates; the sample image processor itself only interprets flat bounded images.

The `refusals` scenario rejects scripted labels before allocation: `refuse plain`,
`refuse structured`, and `refuse internal`. It releases the claimed client id before replying.
Wire tests correct the request under that same id; host tests retain the refused attempt and
submit a corrected request with a new id. No refusal creates a run or spends money.

`queued` waits before allocating until `release-queue` exists, or cancels without a run while
retaining the client-id claim. `reused-null` models an earlier claim without an allocated run.
`reused-run` returns a reference for resource reconciliation; `reused-foreign-run` deliberately
returns another client's id. `known-run-rpc-error` deliberately violates the contract by sending
an RPC error after reporting and saving a run. These are host recovery probes, not conforming
execution examples. None dispatches a provider or spends money.
`queued-resource-error` additionally fails runs-list recovery with a plain RPC error; this must
never be mistaken for a refusal of the original action.

### Estimate ownership probes

The `estimates` scenario exposes a paid `price_test` control target beside a held, free
`mark_band` run. It never calls a provider. `estimate-started`, `release-estimate` and
`estimate-read` coordinate the reader with the test; `estimates.ndjson` records only controls,
not runs. Labels choose a held response, invalid session, plain or structured refusal, or a
broken transport. The latter plus `hold-after-eof` deliberately keeps the test child alive so
shutdown tests distinguish a disconnected pipe from confirmed process exit. The host must
retain its prepared inputs through that boundary. `release-run` completes the independent
free run; none of these controls creates an attempt or incurs provider charges.

The estimate scenario starts with a synthetic $0.20 session budget. Labels `ledger` and `drain`
raise reported spend to at least $0.08 and $0.19 to exercise later ledger observations; they do not charge anything.
Caps above its configured $1 limit receive an `invalid_arguments` refusal, while a positive cap
below the $0.12 worst case returns a cap-refused estimate. Optional ordered `references` are
bound by content fingerprints and coordinates, as is `source`. The
`estimate-declaration-changed` marker changes the paid action's description on the next refresh.
Browser probes use these controls to verify draft/session/declaration invalidation separately
from ledger updates, and that estimates never become history entries.


### Paid dispatch probes

In the `estimates` scenario, `price_test` accepts a paid call as an offline protocol exercise.
It records exact parameters and metadata, allocates a normal run, recomputes the quoted binding,
checks expiry/cap/current budget, and reports a synthetic $0.08 cost against a $0.12 worst case.
It never contacts a provider or incurs charges. Session settlement is atomic; input hashing runs
outside the ledger lock. A cancellation observed before allocation produces no run. Completed
calls preserve source/reference evidence and a journal; no synthetic repair image is promised.
Host tests inspect exact wire metadata, concurrent/sequential idempotence, evidence export,
authentication/origin checks, refusal of altered/unpriced/expired calls and fresh paid retries.

Paid outcome labels extend that same independent peer:

| Label | Simulated outcome |
| --- | --- |
| `paid-stale` | A changed effective model makes the binding stale; zero spend. |
| `paid-cap` | The execution limits check needs $0.60 rather than the quoted $0.12; a $0.50 cap refuses it before spending. |
| `paid-credential` | A synthetic provider 401 costs zero and marks the credential invalid. |
| `paid-failed` | A provider failure reports a known $0.08 charge. |
| `paid-unresolved` | A sent request has no settled answer; $0.12 is counted as unresolved. |
| `paid-full` | A known $0.12 charge exhausts enough budget to refuse another earlier quote. |
| `paid-validate` | A known $0.08 result changes credential readiness from present to valid. |
| `paid-not-ready` | The call's pre-allocation check finds a missing requirement and refuses without claiming a run. |
| `paid-queued` | Waits at `queued` until `release-queue` or cancellation; a cancelled ID stays claimed with no run. |
| `paid-held` | Reserves $0.12 and writes `paid-provider-started`; `release-paid` completes it, while cancellation settles $0.12 as unresolved. |

`paid-credential-state` is a synthetic state marker, never a key. Reads and estimates remain
responsive while `paid-held` waits, with one-second progress heartbeats. Reservations and settlement hold the ledger mutex only for
atomic updates; unresolved spend remains in the session after cancellation. These probes test
host accounting and presentation, not real provider pricing or behavior.

### Render process fixtures

`sample-lab --render-test <mode> <existing-run-directory> [literal-arguments...]` exercises the
host's process boundary independently of MCP. It does not change the run directory. `echo`
reports argument strings, environment variable names (never values) and the working directory;
`hold` stays alive until stdin EOF; `stuck` deliberately ignores EOF until explicitly killed;
`early-error` exits 17 with a synthetic diagnostic; `stdout-flood` and `stderr-flood` exceed the
host's output limits. These are process-test behaviors, not yet conforming server/document views.
V15/V16 add report readiness and document fixtures. No mode calls a provider.

The `renders` scenario also declares `server-probe` and `document-probe`. Their standalone
probe page attempts parent/storage access, top navigation and popups; the server variant also
attempts unauthenticated workspace fetches, form submission and redirects. A test-owned
`report-target` file in the tool working directory selects a loopback canary destination.
No host types, credentials or providers are imported. Browser tests verify actual blocked
operations and that unapproved origins receive no navigation request.

## v0.4 operation peer (M5.01)

`v04_fixtures.rs` owns synthetic operation/task/utility descriptions, resolved results, references,
previews and control examples. Published `table-manners =0.4.0` is now the single exact dependency
for both the live client and independent peers. Schema and cross-document checks live in
`tests/v04_contract_fixtures.rs`. No host types or real tool output construct these fixtures.
Token and Memo are test-only native types. Location is the shared type. Preview fingerprints
come from actual PNG bytes, never from the native record's digest.

`sample-lab <workspace> v04-operations [--allow-dir <abs>]... [--tool-name <name>] [--hold-operation]` now exposes
an independent peer over MCP 2025-11-25. Describe, export, import, resource reads and all six
synthetic actions are callable. An owned serial worker keeps controls responsive and implements
structured progress, queued/allocated cancellation and joined EOF cleanup. Controls never allocate runs or spend; import/export write only the artifact
store/export area. Discovery alone makes no workspace writes.

The probe follows and deduplicates explicit directory grants, requires absolute existing
folders (exit 64 otherwise), publishes effective `allowed_dirs`, and never asks for MCP roots.
No flags means no granted directories. Every process has a new session ID; EOF exits cleanly.
Input messages are bounded to 2 MiB to fit the declared 1 MiB inline file after base64 encoding;
oversized or malformed JSON closes the peer. The decoded inline-file limit stays 1 MiB. Test-only `--tool-name` changes the metadata/resource scheme
so consumers cannot assume a particular tool name. These flags belong to the fake, not Wafflery.

Run `cargo test --locked --test v04_wire` for initialization, no-roots, restart, invalid/symlink/
space-containing grants, no-run control behavior and bounded-input/EOF checks. This manually
framed client uses no MCP SDK; only process spawning uses the host's descriptor-isolation helper.

### Portable store groundwork (M5.01b2a)

`v04_store.rs` is the fake's independent store. Direct storage checks live in
`tests/v04_artifact_store.rs`; `tests/v04_wire.rs` also verifies the reserved controls and
artifact/preview resources through a manually framed stdio client. Control input schemas are
expanded from the published schemas so an MCP client needs no external URN retriever. The fake portable format is a complete
synthetic artifact-resource JSON document: `.bundle` with `application/x-<tool>-token` for Token,
and `.json` with `application/json` for location. This is its own format, not Rascal's bundle.
Whole-record/reference/preview validation and optional SHA-256 checks precede any store write.
The test tool only understands its own synthetic records; it never accepts a preview PNG as a
native artifact. Native types must belong to the same tool; shared locations can change owner
while retaining identity. Memo is stored only by an internal producer and cannot be imported
or exported. Every import is idempotent by identity, including concurrent duplicates, restart and workspace move.

Stored documents and previews are read by identity; absent/corrupt records have structured
binding refusals. Preview PNG bytes match the saved image fingerprint. Exports live separately
under the fake workspace's `exports/`, never a run directory, and repeat byte for byte.
File imports require canonical explicit grants and regular files; each path component is opened
without following symlinks again after the grant check. File records are bounded to 4 MiB and
inline bytes to the declared 1 MiB. Original symlinks resolving inside a grant are permitted;
escapes, missing paths and non-files receive the same refusal without revealing file existence.
Digest failures and malformed records are refused before saving; corrupt stored entries are
reported as store errors on import rather than overwritten. Nothing
in these controls creates runs or spend. Host preservation and path enforcement remain separate
M5 tasks; passing these independent tests does not claim live-client support yet.

### Binding groundwork (M5.01b3a)

`v04_bindings.rs` resolves declared scalar/list operation ports and task image slots using the
published input schemas. Unknown actions, malformed arguments and missing required inputs are
plain invalid-params errors. Reference failures collect every problem in declaration/item order,
with indices only for list ports. Defaults are applied and delivery-only flags are kept separately.
Binding allocates no run, writes no artifact and never reconstructs an upstream operation.
References must name this peer's held record, including its frame and emitted `on` metadata
(optional pixel hashes are ignored). A missing/foreign image reference offers the actionable
remedy of supplying the original image file/inline bytes; native/location references offer import.

This illustrative peer accepts the owned sample PNG and midpoint marker in its synthetic frame;
other image bytes or literal marker geometry produce an explicit invalid-input outcome, never
an example substituted for the request. Native inputs may come from imports, including after the
producer workspace is deleted. File inputs use the same canonical grants as controls. File
fingerprints/dimensions/frame must match `expect`; optional pixel hashes never override matching
file identity. Image artifact references remain references in resolved arguments; file/inline
images become `{expect}`; literal locations stay literal. The store can hold the owned image
internally and serve its original bytes as its preview, but cannot import/export it as a native
Token. Tests live in `tests/v04_bindings.rs`; b3b/b3c below implement run outcomes and wire execution.

### Resolved outcomes (M5.01b3b)

`v04_results.rs` executes the illustrative steps from resolved inputs: midpoint location,
native Token creation/amount changes, ordered list pass-through, and a preview-free Memo.
It produces every action's invocation and mirrors port artifacts into image/location profiles.
Original input bytes remain intact. Output previews honor resource/inline/path selection with
matching encoded-byte fingerprints; delivery options never affect the invocation/input identity.

The result builder accepts a cancellation predicate and checks it between inputs and outputs.
Early cancellation performs no work; later cancellation retains only completed entries. Failed
preview writes become `io` error results with the resolved invocation and a plain reason.
Final elapsed time updates preserve the outcome and derive the finish timestamp from start plus
monotonic elapsed time; invalid-input identities also exclude delivery options. Error/cancelled settlements
keep completed outputs, while unresolved invalid inputs have null invocation and empty artifacts.
Direct executor tests live in `tests/v04_results.rs`; c3b2 below verifies notifications,
active workers and cancellation over stdio.

### Run records and claims (M5.01b3c1)

`v04_runs.rs` owns a session's atomic correlation claims and the independent peer's run files.
Allocation writes the reference/action and journal before publishing a running entry. Final
results are immutable and atomically written last; their original session reference remains
readable after a workspace move or restart. A new session starts with no claimed client IDs.
Only pre-run refusals release claims through a private ownership handle; refused duplicates
receive no handle, and stale handles cannot release or allocate another call's claim.
Cancellation while queued keeps its claim without a run. Read resources require a published
allocation. After relocation, reads update only `run.dir` to its current local path; stored
results and all four reference identity fields remain unchanged. A failed final write can leave
a repeated terminal publication attempt in the private journal; only `run.json` establishes
terminal status. This fixture does not implement crash recovery.
Two-entry pages encode a `(started, id)` position, so new runs cannot shift later pages.

Direct state tests in `tests/v04_runs.rs` cover concurrent claims, refusal release, failed
allocation, running/final reads, immutable completion, mismatched references, relocation and
paging across equivalent timestamps with different RFC3339 spellings. Wire dispatch now uses
these records alongside the c3b2 progress, cancellation and EOF checks below.

### Callable schemas (M5.01b3c2a)

`v04_schemas.rs` builds each synthetic action's callable input schema from its declared parameters
and scalar/list ports. The resolver uses the same schema intended for tools/list, including
literal image/location alternatives and delivery options. Pinned, acyclic contract references
expand locally without external retrieval. Reference sibling keywords stay at their original
level, preserving closed-object and `unevaluatedProperties` validation. This is an owned-fixture
helper, not a general resolver for arbitrary tool descriptions. Four schema tests exercise every
action, port cardinality, undeclared fields, reference siblings and the published control inputs.
The wire catalogue and call validation now use this same builder.


### Wire execution (M5.01b3c2b2)

`v04_calls.rs` connects owned correlation claims, input binding, execution and saved outcomes.
All six described actions appear in tools/list with complete input schemas. Binding/shape
refusals return JSON-RPC errors before a run and release only their owned claim. Invalid resolved
inputs and execution failures return saved structured results with `isError: true`; completed
calls retain their claim and a reused ID points at the earlier run. Every allocated result and
journal is readable by resource URI, and runs paginate across sessions. Stdio defaults to path
previews; image content blocks are returned only when explicitly requested. The original PNG
and artifact identities remain authoritative. Stored path previews are local locators; relocation
does not rewrite them (resource previews and portable artifact controls avoid that dependency).

The initial eleven wire tests cover an independently renamed Locate → Stamp → Soften chain, ordered
many-input bindings, refusal/reuse semantics, invalid and I/O outcomes, saved resources/paging,
opt-in image blocks and the earlier control/no-root/transport checks. All work is synthetic and
free. This slice originally dispatched synchronously; c3b2 below adds the owned worker,
progress, cancellation and EOF ownership before host adoption.

After allocation, recoverable journal or image-content delivery failures become saved I/O results,
retaining resolved invocation and completed outputs. Image blocks are prepared before publication;
the text, structured result and run resource agree on the exact saved outcome. Earlier invalid or
cancelled outcomes retain their primary status when a later journal or delivery failure occurs,
with a diagnostic. A failed execution journal and a
failed final journal append may each add evidence; diagnostic count is not an API. Missing journals
remain unreadable even when run.json is writable. Only failure to validate the allocated record or write final evidence itself is a
server error; the allocated claim remains held. Direct dispatch fault tests cover both failure phases.

### Cancellable dispatch boundaries (M5.01b3c3a)

Acceptance now claims and validates before the execution permit; allocation is a separate phase.
A per-request cancellation guard serializes cancellation with allocation and final publication.
A queued cancellation keeps its claimed ID and creates no run. An allocated cancellation with
resolved invocation writes `cancelled`, retains completed outputs, and yields no reply. Cancellation
after final publication cannot rewrite its outcome. Allocation/execution callbacks provide the
worker's deterministic hold points without timers. The c3b2 wire path now uses these same
boundaries for cancellation notifications and EOF worker cleanup. Once invocation resolves,
publication is the terminal boundary: cancellation before it saves `cancelled`, retaining any
prior execution failure reason in notes. An input rejection before resolution remains `invalid`
with null invocation even if cancellation arrives before saving; the cancelled request still
gets no MCP reply. After publication, cancellation cannot rewrite the outcome. The guard
also suppresses storage-error replies for cancelled requests, writing a stderr diagnostic instead;
the retained claim still prevents redispatch. Such a storage failure can leave a running entry
without final evidence. Only an invalid run may have null invocation.
Panics are not recovered by this synthetic dispatcher.

### Owned execution worker (M5.01b3c3b1)

The worker core accepts at most 32 active/queued calls, claims and resolves each before queueing,
and executes one at a time. Duplicate request IDs and queue overflow refuse before claiming a
correlation ID. The reader continues serving controls independently. With the explicit hold
fixture enabled, completed synthetic work waits for `release-operation` in its owned root; time
never completes the hold. Progress includes the structured reference without `dir`, advances on
journal stages and every second while held. Queued heartbeats omit the run reference until
allocation and retain their increasing counter when execution starts. The held job supplies
that wait tick; unheld synthetic steps are immediate and use stage notifications. No progress is sent without
a token or after cancellation. A failed output
callback requests cancellation. Shutdown cancels all accepted work and joins before returning;
queued cancellation creates no run and allocated cancellation preserves completed outputs.
Direct channel tests exercise this core; c3b2 connects it to stdio and EOF.

### Stdio lifecycle (M5.01b3c3b2)

The independent peer now routes actions to the owned worker while its reader serves describe,
artifact export/import and every resource independently. It handles `notifications/cancelled`
without replying to the cancelled request. All output is serialized as complete JSON lines;
write failures return false to the worker and request cancellation. Stdin EOF cancels queued and
active requests, joins the worker and exits only after writable cancelled evidence is saved.
The same cleanup runs on reader errors through owned Drop. No MCP roots requests are made.

Use `--hold-operation` only for lifecycle tests: after synthetic outputs finish, the run remains
running until `release-operation` exists in the peer's owned root or cancellation arrives. The
release file is one-shot for that root; no timer completes work. Six additional child-process
tests cover controls and artifact resources during a hold, structured running/queued heartbeats,
queued ID retention without a run, allocated cancellation with output salvage, EOF joining, reader-error cleanup before stdin closes, and cancellation after input rejection
that preserves invalid status/null invocation without sending a reply.
The live host now accepts v0.4; operation workspace support follows the M5.03 contract cutover.

Lifecycle clients must drain stdout through child exit or close the read pipe; keeping an unread
pipe open can block normal output backpressure and therefore shutdown joining. Cancelled queued
RPC IDs and admission slots remain occupied until the serial worker consumes those entries.

### Explicit host directory grants (M5.02a)

The old-flow peer now reads repeated `--allow-dir` arguments instead of asking for roots. Its
synthetic `initialize.json` and `allowed-dirs.json` records expose handshake/grant evidence for
boundary tests, without configuration or environment values. Direct wire probes pass their own
grants; host probes prove that only input grants and the dedicated execution area reach argv.
The production host's structural validator now accepts only v0.4; both independent peers use
the same explicit path rule. `tests/directory_grants.rs` checks registration
rejection/canonicalization, literal argv, no roots capability and independent evidence reads.

### Frozen access and hostile observations (M5.03)

The `estimates` scenario accepts `--config <path>` with an optional JSON `allowed_dirs` list.
Startup intersects canonical configured directories with the explicit launch grants. A valid
object without that field adds no restriction; an unreadable or malformed configuration, or any
invalid directory entry, denies every caller path. An empty list also denies all paths. Effective
grants and their configuration identity stay fixed until a new process starts. Inline image bytes
still work without filesystem grants (`tests/control_wire.rs` covers that boundary). No real
provider or credentials are used.

For boundary tests only, `<workspace>/resource-grants.json` and `estimate-grants.json` replace the
`allowed_dirs` value in the corresponding session response. Malformed JSON supplies `null` so the
host must reject it. These files do not change actual access checks or the ledger. Removing them
restores the real observation. `tests/session_access.rs` exercises both rejection channels,
configuration freeze/restart, denied copied inputs and preserved spending when a directory vanishes.
