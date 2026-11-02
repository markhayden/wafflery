# Independent offline peer

`sample-lab` is Wafflery's owned test tool. It is explicitly illustrative; it does not stand
in for Rascal's repair operations or Squint's judgments. Its wire implementation does not use
rmcp or Wafflery's coordinator/client. It may use the published contract's data/schema crate.

The fixtures target published `table-manners =0.3.0`. `fixtures.rs` builds deterministic
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
2025-11-25 initialization, roots, tool pagination, both path and inline image calls, expected
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

Select a local binary explicitly (never discovered or installed automatically):

```sh
WAFFLERY_RASCAL_BIN=/absolute/path/to/rascal cargo test --locked --test rascal_smoke -- --ignored --nocapture
```

This uses the same host coordinator, fresh no-paid check, exact contract-version allowlist,
image fingerprints and evidence capture as the fake. The child inherits no environment,
receives no config or filler, and gets temporary workspace/run directories. It checks a real
`seam` result, zero spend, verified outputs/journal and unchanged original bytes. `mask_rect`
requires a location binding, which is outside the foundation's image-and-parameters form.

The seam needs explicit equirectangular/wrap image coordinates. The host's optional `frames`
field records that shared image-reference metadata alongside ordered asset bindings. Ordinary
images default to flat/bounded. This metadata is validated using the published image-reference
schema; the host never interprets a tool's private projection parameter. The smoke explicitly
sets Rascal's declared parameter as well. A flat seam can complete without producing an image,
so that alone is not a successful output smoke.

Missing binaries or unsupported contract versions print an explicit `SKIPPED` reason. Other
failures remain failures; do not loosen the version, spend or fingerprint guards to pass them.

## Optional real Rascal browser journey

```sh
WAFFLERY_RASCAL_BIN=/absolute/path/to/rascal bun run --cwd ui test:browser
```

The optional Chromium/WebKit journey uses explicit registration and the same generic UI as any
tool. It checks a real seam mask, saved inputs, recorded call, image evidence download, independent
example loading, and history/output preservation across a host restart. Without the environment
variable those two cases are reported skipped. Choose a stable contract 0.3.0 executable; rebuilding
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

Use an explicitly selected, stable contract 0.3.0 Squint binary built with `--features testkit`
at `3e6223a` or a later compatible commit. The coordinator probe owns config/script/files, clears
its child environment, and supplies no credentials:

```sh
WAFFLERY_SQUINT_BIN=/absolute/path/to/testkit/squint cargo test --locked --test squint_smoke real_squint_scripted -- --ignored --nocapture
```

This verifies discovery, truthful free metadata, a source-fidelity scripted result, zero spend,
recorded call, complete evidence and unchanged original images. The verdict is fixture data,
not a model judgment. Other use cases and reference-list bindings are not certified by this test.

With both `WAFFLERY_RASCAL_BIN` and `WAFFLERY_SQUINT_BIN` set to explicit stable binaries,
`bun run --cwd ui test:browser` includes the two-tool journey: Squint check, inspect/export,
example reload without execution, switch to Rascal and run a seam, restart, then restore Squint
history without mixing results. Without both variables those cases are skipped.

The separate ignored `real_squint_estimate` probe now passes on `3e6223a`: it preserves an
empty readable journal, resolving [the follow-up](../docs/handoffs/squint-estimate-journal.md).
Both probes require an explicitly selected testkit binary. [Original discovery blockers](../docs/handoffs/squint-discovery-blockers.md)
are resolved. `scripts/dev-pipeline.ts` starts the same isolated offline setup through Squint's H1 launcher.

## v0.2 association and cancellation scenarios

The peer publishes a fresh session UUID per process and echoes client correlation IDs in progress,
run resources and results. The host captures the SDK-assigned progress token at the wire boundary.
`slow` exercises cancellation with progress association; `no-progress` and `paged` exercise
runs-list recovery. `unassociated` deliberately withholds evidence. `duplicate-pages`,
`conflict-once` and `endless-pages` verify that contradictions or incomplete searches keep the
outcome unconfirmed and the reservation held. These negative peers deliberately violate the
contract and are not conformance examples. `session-reset` proves a new ledger cannot silently
authorize dispatch. The same production may_spend guard still rejects paid actions.

## v0.3 preparation (V01)

`v03_fixtures.rs` owns descriptions for `sample-lab` and `test-lens`, a ready offline action beside
an independently configured paid action, and session/estimate/refusal examples. These fixtures
perform no provider calls. Paid host dispatch additionally requires an owned priced preparation.
The preparation tests validate independently of host-generated types.

V02 replaces the temporary V01 schema archive with the exact contract dependency's embedded
schemas. Fixed timestamps/UUIDs in fixture examples are test data; serving processes provide
fresh sessions and live expiry. The temporary path points to reviewed table-manners e503a23.

The `v03-ready`, `v03-lens`, `v03-needs-setup`, `v03-unavailable` and `v03-budget` scenarios expose
those objects over MCP 2025-11-25. They are **control-only preparation probes**, not complete
execution peers: describe, estimate, session/runs reads and discovery work; action execution is
explicitly unsupported until its owning task. The normal/held execution scenarios also use v0.3.
`price_test` estimates are deterministic and local, with a scripted $0.12 worst case and $1
configured cap. Delivery flags do not affect the binding. Each process gets a fresh session;
no estimate allocates a run, claims a client run ID, reserves money or changes the ledger.
The wire tests validate returned objects against the dependency’s embedded schemas, compare the ledger
before and after controls/refusals, and verify the workspace remains empty.

V02 discovery negatives (`v03-legacy`, `v03-no-estimate`, `v03-no-cap`) intentionally violate
one live boundary. The older `paid`/`becomes-paid` scenarios remain dispatch-guard probes:
they advertise the reserved estimate control but do not implement a paid execution flow.
Only the control-only v03 scenarios currently implement estimate responses.

## Readiness scenarios

`readiness` and `readiness-held` describe an offline action with an optional missing filler,
beside a paid action whose ordinary unprepared submission the host blocks. `credential-state` under the peer
root selects present/valid/invalid/missing/unknown on the next description, without reading any
credential. Only requesting the optional filler produces -32602 not_ready, before claiming the
client ID or allocating a run; an omitted filler remains usable. Wire coverage reuses that
released ID successfully. The held variant uses the existing release-run gate for concurrent
refresh tests. The paid action is discovery-only; estimate execution remains in v03-control.

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
