# Wafflery engineering draft

Status: embedded SQLite choice accepted (Q18); detailed storage and process design proposed,
2026-10-30. Not an approved build plan or an
implemented system. [SPEC.md](SPEC.md) owns product decisions; [ACCEPTANCE.md](ACCEPTANCE.md)
owns observable outcomes. [ADR-0002](docs/decisions/0002-local-library-and-host.md) explains
the engineering recommendations here. This draft does not invent additions to table-manners.

The [foundation runtime](docs/design/foundation-runtime.md) specifies the selected stack,
private browser API/session boundary, repository layout, planned commands and verification.
[PLAN.md](PLAN.md) scopes M0–M3 so later tool agreements do not block independent host/UI work.

## Scope and source boundary

Design for one user on macOS, one active local library, one host, and independent Rascal/Squint
execution. Linux is deferred. No persistent job service, automatic retries, batch queue, or
cross-tool workflow engine. Closing a browser is different from stopping the host.

Historical integration check, 2026-11-01: published table-manners v0.2.0 (tag `3c8b55c`) was adopted with isolated
committed Rascal `27ef3b5` and Squint `3e6223a` builds. MCP stays 2025-11-25 and H1 is unchanged.
The host records validated session ledgers and stable client correlation IDs, checks structured
progress/result associations, and performs bounded cancellation reconciliation. It does not
reset allowances, resubmit missing runs or restart tool processes automatically. See the
[v0.2 integration plan](docs/plans/table-manners-v02.md) and [source inventory](docs/integration-sources.md).
Native artifact preservation, report readiness, and generic provider configuration still need
the [owner agreements](TOOL-REQUIREMENTS.md). Paid dispatch remains disabled until those guards
are implemented; the owner's permission to spend is already recorded.


## Storage ownership

Default library: `~/Library/Application Support/Wafflery/Library`.
The parent Wafflery directory contains settings and private host discovery/lock files so
changing library location cannot accidentally create two active hosts. Only local storage is
supported initially; network/shared live libraries are outside scope.

Proposed layout, not an existing filesystem:

```text
Wafflery/
  settings.json                 selected library path, non-secret preferences
  host.lock                     held for the lifetime of the host
  host.json                     private discovery record; not proof a host is alive
  tools/<installation-id>/       stable declared tool-owned workspace/run storage
  Library/
    library.sqlite3             metadata, references, persisted outcomes
    assets/sha256/<prefix>/<hash> immutable owned bytes
    staging/<id>/               incomplete imports and exports
    execution/<attempt-id>/     materialized inputs and temporary working files
```

The database stores logical IDs and library-relative owned paths. Original absolute paths are
provenance only. A tool's absolute paths/IDs remain tool references and must not be silently
rewritten as portable host references. Settings contain no provider credentials.

Keep tool workspaces outside the relocatable library so moving owned history/assets does not
require rewriting opaque tool paths. Display both storage locations and their usage. Moving the
library does not move tool workspace bytes; moving those requires a tool-supported procedure.
This is a storage-location change on the same machine, not a portable backup or migration.

| Record | Minimum responsibilities |
|---|---|
| Attempt | Host ID, tool/session/version, action, immutable submitted arguments, input asset bindings, estimate/cap context, timestamps, execution state, reported result/spend, preservation state |
| Asset | Byte digest, byte length, media/type metadata where declared, relative path, verification state |
| Asset reference | Owning attempt/example and role; native type/frame/identity remain distinct from the storage digest |
| Example | Name/notes, source attempt provenance, copied invocation snapshot, independent references to required assets |
| Baseline | Task/action context and selected saved attempt; deleting that attempt clears its baseline selection |
| Tool session | Host session identity, installation/configuration identity, start/end, tool-reported budget observations; no credentials or invented provider balance |

Effective configuration is recorded only through an agreed safe tool interface. Do not copy
whole configuration files or process environments into history. Preserve declared journal/report
data under that same non-secret contract; diagnostic stderr is not automatically exportable data.
Bound diagnostic capture and mark truncation; exclude credentials and authentication headers.

## Child process environment

Every tool starts with a cleared environment. Wafflery copies only inherited `HOME`,
`XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `TMPDIR`, `LANG` and `LC_ALL` when present,
and supplies the fixed system `PATH` `/usr/bin:/bin:/usr/sbin:/sbin`. This supports default
configuration lookup without inheriting unrelated provider credentials or shell settings.

The environment selector additionally permits declared `credential` and `env` requirement names
matching `[A-Za-z_][A-Za-z0-9_]*`. These cannot override base variables or forward `WAFFLERY_*`,
`DYLD_*` or `LD_*`. Excluded and non-identifier names remain valid displayable requirements; the
host does not reject the description or attempt to resolve credentials held elsewhere.
Missing variables remain absent. Values are copied only from the host environment directly into
the child command; registration, logs and saved evidence never receive those values, including
non-secret `env` values. Requirement names alone are safe to retain.

Startup first runs an MCP discovery probe under the base environment, using the registered
config path or the tool's default lookup. It closes the probe before starting one serving
process with the declared credential/env names. Only the serving process's session is published.
A probe that fails to close remains owned for deliberate shutdown; no serving process starts
and no force kill is automatic. A failed discovery also prevents serving startup.

If the serving description adds credential/env names, setup remains visibly incomplete, including
after a refresh. Discovery does not loop or restart automatically to acquire more variables.
External environment changes require
relaunching Wafflery; restarting a tool cannot refresh its parent process's inherited environment.
The host does not read configuration contents or load shell profiles to discover credentials.

## Capturing inputs and results

1. Import a user-selected file into host staging, hash the copied bytes, and validate its limits
   and expected identity. Reject an unstable capture when detectable; the retained bytes define
   the submitted input. File uploads need no permission to read the user's entire source folder.
2. Flush and publish a completed asset without overwriting an existing digest. Commit its
   ownership reference only after the file exists. Byte-identical assets share one host copy.
3. Materialize execution inputs from owned bytes with isolated copies or copy-on-write copies;
   never hard-link mutable originals or give tools mutable access to the canonical asset store.
   Keep filenames/extensions where required without accepting path traversal from supplied names.
4. Capture the action, exact arguments, available effective defaults, tool/contract versions,
   and asset bindings. Validate the current estimate against these inputs before dispatch.
5. Persist the attempt and its submission identity before sending the MCP call. A crash between
   recording dispatch and actually sending it can leave an uncertain attempt; do not guess or retry.
   Always include a unique MCP progress token. Persist the mapping from host attempt to session,
   request ID, and progress token before dispatch; bind the tool run resource/ID when received.
   Read the first progress run reference using agreed wire semantics (C4), not message scraping.
   A final result can supply the mapping when it arrives first. Conflicting mappings are protocol
   errors, never permission to attach another run's evidence.
6. On terminal result/resource, persist the reported outcome first. Preserve declared images,
   intermediate artifacts, journal, and report dependencies through supported interfaces.
   Store completed evidence even for failed/cancelled runs when provided.
7. Track preservation separately for contract evidence, native dependencies, and report data.
   Mark a category complete only after its required assets are verified and referenced.
   Capture failure leaves the execution outcome intact with a separate incomplete-preservation
   state and actionable missing items. Never claim a successful tool result means history is safe.

Execution states: `preparing`, `running`, `cancelling`, and terminal `completed`, `invalid`,
`error`, `cancelled`, or host-observed `rejected` / `interrupted`. `rejected` means local validation
or a correlated pre-run protocol rejection, not an uncertain transport failure. `interrupted`
does not claim the tool stopped or that no money was spent. Neither is a fabricated tool result.
Each preservation category has state `pending`, `complete`, `incomplete`, or `unsupported`.
Tool result bodies remain available as data, not parsed into private tool-specific host logic.

If cancellation/connection loss occurs before the tool run reference arrives, retain the known
request mapping and unknown outcome. Never match run-list entries by action, time, or input
similarity. While the session remains alive and may be working, retain its execution reservation;
offer explicit stop/restart if association cannot be recovered. A dead session leaves an
interrupted attempt. Additional correlation semantics are C4, not an invented v0.1 field.

The v0.1 contract-evidence category includes the terminal result JSON, journal resource stored
opaquely with its content type, every `images.out` file, and the result's location data. Verify
image bytes against `file_sha256` and decoded stored dimensions. Resolve location image references,
including mask renderings, against captured inputs/outputs by declared identity. A fingerprint
alone is not a locator: if required bytes cannot be resolved through declared bindings, record
missing evidence; do not search unrelated files or invent a resource URI. Return-path, inline,
and advertised resource mechanisms remain those of v0.1. Preserve structured result data without
claiming original JSON whitespace/order when obtained via MCP rather than the run resource.

Label this category "Contract evidence saved" when complete. Do not call the whole run fully
preserved while native dependencies/report data are unsupported or unknown. C2/C3 must declare
those sets before Q15's full preservation target can be verified. Raw journals/results follow
the non-secret data boundary above; undeclared secret-bearing fields cannot safely be exported.

Once execution is confirmed terminal and preservation/export no longer needs materialized
inputs, remove those disposable copies. Keep references to immutable assets in retained records.
After a crash, do not sweep execution directories that a surviving child may still use; first
establish that their owner has exited. Ordinary isolated copies are sufficient initially;
copy-on-write is an optimization, not a required APFS-specific implementation.

SQLite transactions cover metadata only. Files are staged and published before their references
are committed; failures can leave unreferenced files, not a falsely complete saved run. A host
restart may remove abandoned staging files once no live owner exists. Unreferenced committed
assets are reported as reclaimable and removed through explicit cleanup. Do not disable durable
database writes to make tests faster. Exact journal settings/library pins are selected later.

## Native artifacts and tool-owned reports

Prefer a tool-declared export/import or reconstruction mechanism for native dependencies. A
rendered mask is a preview of a native Mask, not its replacement. Keep displayability, native
reusability, and portable export as separate capabilities on each saved artifact.

Prefer preserving report source data and invoking the owning tool's renderer on demand. This
avoids depending on a forever-running viewer. It still requires an agreed way to restore the
report's dependencies and compatible tool/version. A declared self-contained report export may
also be retained. Copying arbitrary run directories is not a generic preservation protocol.

If native artifacts remain dependent on a managed tool workspace, label that dependence and
protect it from cleanup while referenced. Do not promise portable export or location changes
for that workspace until the tool declares support. Missing declarations are C2/C3/R3 blockers
for those capabilities, not permission to implement tool-specific filesystem scraping.

Generic previews and saved metadata remain readable when a tool is missing. Native replay or
tool-rendered reports can require reinstalling a compatible tool; explain that locally.

## Deletion and changing storage location

Keep runs/examples until explicit deletion. Compute references within the host before cleanup;
do not rely on a cached count that concurrent example creation can invalidate. Active attempts
and exports hold temporary references. Block deleting an active attempt until it terminates.

Deletion first removes selected records/references transactionally, then removes unreferenced
owned assets. Recheck references under the host's storage lock before unlinking. Failed deletion
can leave reclaimable files; it must not break surviving runs/examples. Baselines do not prevent
an explicit run deletion; the UI explains that the pin will clear. Examples retain their assets.

Never recursively delete unknown tool workspace files. Cleanup of native tool stores needs a
declared ownership/deletion interface. Show opaque tool storage separately in disk usage;
reclaimable bytes must not include space Wafflery cannot safely release.

Relocation is an idle maintenance operation: no active runs, imports, viewers, or downloads.
Close tool processes and the database cleanly, copy into an empty destination, verify owned files,
then atomically change the configured library path. Keep the old copy until explicit removal;
failed copying leaves it selected. Do not merge two libraries. Tool workspaces retain their
stable location outside the library. Explicitly disclose that their storage is not being moved.
Verify no opaque native dependency points into a disposable/relocated execution path; any such
unsupported dependency blocks relocation until resolved, without blocking ordinary v0.1 runs
whose owned inputs/outputs and stable workspace bindings suffice.
The library UUID and content digests remain stable. Final relocation tests depend on C2/H1.

## Host and tool processes

Tool `playground` subcommands only locate/install and invoke the Wafflery executable with
the agreed tool-selection context. They do not read locks, authenticate a host endpoint, mint
browser secrets, or open the final browser URL. The Wafflery binary owns all those details.

The Wafflery launcher resolves the configured library and tries to attach to the existing host. The
host owns an OS lock for its entire lifetime; a stale discovery record alone cannot authorize
attachment or killing a PID. Authenticate the live endpoint and check host protocol/library
identity. Simultaneous launches must converge on one host. An incompatible live host is an
explicit restart requirement, not a reason to launch a second writer or upgrade it mid-run.

If no host owns the lock, start the same binary in its internal host mode, detached from the
launcher's terminal lifetime. Wait for a bounded readiness handshake before opening the browser.
The [H1 proposal](docs/handoffs/H1-launcher-proposal.md) now defines the public launcher boundary;
the private handshake is implemented and tested in F05. Do not install a login agent or automatic restarter.
Keep development foreground mode separate from the installed user's launch experience.

The host binds loopback on an available port, serves embedded assets, and owns all browser
sessions. Use a private single-use bootstrap secret to establish an authenticated browser
session; do not expose it in ordinary logs or pass it to report viewers. Validate Host/Origin
and mutation authorization; loopback alone is not authentication. Detailed HTTP/session types
and browser embedding policy are a remaining engineering section.

Start tool MCP servers lazily. Use executable/argv/configuration declarations, not a shell
command string or a tool-name switch in host code. Host-managed workspace/run roots must be
supplied through an agreed startup descriptor (H1); v0.1 does not define that descriptor.
Never silently adopt or mutate an unrelated existing tool workspace.

Each configured tool has one server session and at most one active execution. Do not queue runs
in the first release; show the active run and disable new submissions for that tool. Other tools
may run concurrently. Reserve execution atomically at the host so two tabs cannot race through
the same Run button. Repeat HTTP submissions with the same attempt token return that attempt;
they never submit a second MCP call. This does not promise exactly-once provider execution.

Planned v0.3 behavior (PLAN V09–V13; not implemented): prepare exact arguments and materialized
inputs, then estimate outside the host execution reservation. Estimates remain responsive while
an action runs; they neither consume a run-dispatch permit nor create attempts. The exact allowed
quote, positive cap and binding are checked again under the host's execution reservation before
dispatch. Tool-side caps and budgets remain authoritative; a quote is not a reservation.
Keep one active execution per tool and reject a second submission as busy. Controls do not wait
for that slot. The former Squint behavior that created estimate runs describes v0.2 integration
history only: v0.3 prohibits it. Until the approved paid slice lands, dispatch stays offline-only.
Changing config or restarting a tool invalidates estimates and ends its old budget session.
Show new session boundaries explicitly; never restart automatically to replenish an allowance.
The authenticated restart control requires explicit confirmation and an idle execution slot.
The restart lifecycle remains owned if its HTTP requester disconnects or cancels.
It serializes with submission, discovery and shutdown, closes the owned child, then repeats the
config probe before atomically publishing the new client and declaration. A failed close retains
ownership and starts nothing; failed startup leaves a disconnected state for deliberate recovery.
Active attempts retain their original client for cancellation and reaping. Reaping cannot release
a reservation while its execution worker is still saving evidence or cleaning inputs. Saved
unknown outcomes remain unknown after a new session starts; they do not permanently block it once
the previous child has exited. Description revisions include a process identity so old drafts
cannot dispatch against a replacement. Restart changes notify all attached tabs.

Recheck may refresh discovery without restarting a live session. If applying configuration needs
a restart, label that action "Restart tool session" with its budget effect and require it to be
idle (or explicitly stopped). The ordinary recheck action must not reset the session silently.
Do not present a per-process budget as a daily budget or provider account balance. C4 defines
the final tool-facing semantics, including uncertain spend after interruption.

## Stop, quit, and interruption

- Closing a tab does nothing to process lifetime. A reopened tab fetches current state and
  subscribes to updates; event replay is unnecessary when a fresh snapshot is authoritative.
- Stop sends MCP cancellation once and shows cancelling. Under current v0.1, cancellation before
  a result normally has no usable run reference: show "Cancellation requested; final outcome
  unconfirmed" as an expected status, not an execution error. A cancelled call has no ordinary MCP
  result; read its terminal run resource when its identity is known. Before association, follow
  the unknown-outcome rules above. A missing resource is not proof of zero spend.
- Quit blocks new submissions, cancels active work after the accepted confirmation, closes
  report viewers, and gracefully stops owned tool servers. If a child will not stop, show the
  condition and offer explicit force quit. A transport timeout must not fabricate cancellation.
- After a host crash/restart, use persisted history. Attempts lacking a committed terminal
  outcome are interrupted; preserve already saved evidence and label uncertain spend. No
  automatic retry, checkpoint recovery, or provider-job polling is required.
- Process ownership must avoid lingering children and accidental termination of unrelated
  processes. Use held process handles/explicit shutdown and test parent-death behavior. Exact
  macOS detachment and child-exit implementation remains part of the runtime selection.

Start report viewers on demand under the host's ownership using the agreed ready-URL protocol.
Keep at most one viewer per run. Use an idle timeout refreshed by visible inspection activity;
no durable per-tab lease system is needed. Closing/disconnecting eventually allows the timeout
to stop the viewer, and quit stops all viewers. Reopening starts one as needed. Viewer failures
affect report availability, not the tool result. Exact timeout is a runtime setting to specify.
Do not launch viewers by parsing human stdout or serve arbitrary tool HTML on the authenticated
host origin. Framing/authentication remains C3 plus the browser security specification.

## File access boundaries

Tools receive explicit MCP roots for materialized inputs and agreed managed workspaces. The
canonical asset store and unrelated home/project folders are not implicit input roots. The
host validates returned file paths against known output roots and rejects symlink/path escapes;
the details must avoid check-then-open races. Resource/inline results get size limits and hashes.

MCP roots express allowed paths; they do not sandbox a locally installed executable. Tool
binaries are trusted local programs. Avoid claiming OS-level isolation that is not implemented.
Host APIs use asset IDs rather than unrestricted filesystem paths. User-selected imports and
explicit output destinations are the only ordinary filesystem entry points.

Asset routes must not grant imported bytes the application's script authority. Only verified,
allowlisted raster formats may be served inline with their detected Content-Type and
`X-Content-Type-Options: nosniff`. Serve SVG, HTML, and other unsupported formats as attachments
with a non-executable content type; never inline them on the authenticated application origin.
Use a restrictive UI Content Security Policy and route-specific attachment/preview rules.
Structured JSON/text inspection uses escaped UI text, not raw HTML insertion. Tool report HTML
continues to require a separate isolated origin and the C3 framing/authentication agreement.

## First fake-tool milestone boundary

Begin with valid v0.1 descriptions/forms, input snapshots, output images, recorded calls,
history, examples, and comparisons. Show unclassified advertised calls under a neutral Actions
catalogue in this development milestone; do not pretend utilities are Operations or infer tasks
from names. Reviewed Playground/Operations components can be exercised with explicitly labeled
UI fixtures in Storybook, without emitting fictional table-manners fields.

Add readiness/classification/estimate mapping, native bindings, runnable exports, and report
viewers once their owner-reviewed schemas/interfaces exist. Progress-to-run wire association
also requires C4 clarification; the first milestone can exercise result-based association and
the explicit unknown-outcome path without inventing a progress extension. These milestone
limits do not shrink the agreed first-release acceptance criteria. Add an opt-in no-config Rascal
`mask_rect` smoke at M2 after the tested no-paid dispatch guard; its exact live description and
contract version must pass the same validation. Squint already has a testkit scripted-provider
path, but it must not be used to bypass a live `may_spend: true` declaration in this milestone.

V02 accepts exactly 0.3.0 using the authorized temporary dependency at table-manners e503a23
until publication. Older saved records remain readable without migration or replay. Paid
actions must declare a positive max_spend_usd and expose the reserved estimate MCP control,
which is excluded from the Actions catalogue. Paid dispatch remains blocked. V0.3 declares
list slots; editing and dispatch for them land in V07. MCP roots remain
specific to 2025-11-25; a future revision requires its own path-access decision.

First-release history records Wafflery-started attempts. Tool run resources support reading
those attempts, not heuristic adoption of unrelated runs. Automatic import of prior CLI/agent
history is deferred; a later explicit import must label unavailable dependencies honestly.

## Verification before claiming these behaviors

Use offline fake-tool fixtures for duplicate clicks/tabs, shared serial limits, changed estimates,
cancelled requests without MCP results, missing terminal resources, malformed paths/symlinks,
oversized outputs, and a server dying after dispatch. No paid calls are needed.
Include cancel-before-first-progress, conflicting run IDs, pre-run protocol rejection, estimates
while busy, and configuration recheck without an implicit budget reset.

Exercise real filesystem faults around asset publication and metadata commit; full disk, failed
copies/deletes, shared example references, original-file mutations, relocation failure, and
missing native dependencies. Verify that incomplete preservation never looks complete.

On macOS, test simultaneous launch, terminal/tab closure, reconnect, explicit quit, child death,
stale discovery records, and report idle shutdown/reopen. A killed host must not trigger resubmission. Test
authentication/origin rejection in real browsers. These are planned tests, not completed checks.
Include HTML disguised as an image and SVG script payloads through both import and tool outputs;
neither may execute with host-session authority.

## Remaining integration and release work

- Agree the now-drafted H1 launcher proposal with the tool owners; registration/installer subgates
  remain separate. C2/C3 preservation/report examples and R3 need actual delivery milestones.
- Review the [C4 v0.2 proposal](docs/handoffs/C4-v0.2-proposal.md) before the contract tag;
  Wafflery owns advancing this request, not passively waiting on it.
- Verify the selected foundation dependencies/tooling in M0; browser API, limits, event/session
  interfaces and commands are specified in the linked runtime design. Final report isolation
  remains dependent on C3.
- Establish release macOS minimum/architectures and installer/bootstrap verification at M7.
- Review the scoped foundation plan before implementation. No application scaffolding is
  implied by these documentation changes.

### v0.3 readiness controls

`POST /api/tools/{id}/refresh` is an authenticated same-origin control. It rereads description
and session without taking the execution permit, updates only live discovery state, and emits a
workspace update. A per-tool discovery mutex serializes reads through publication; submission
holds that same owned guard through preflight and the actual transport write, then releases it
before waiting for the tool result. A concurrent refresh cannot replace authorization between
validation and dispatch. Saved attempt declarations stay immutable. UI drafts survive refresh.
A valid -32602 `not_ready` response with no prior run reference is a terminal rejected attempt,
not an interrupted provider run. Preserve its raw error and requirements without fabricating a
run/result, then release local execution inputs and the reservation. Malformed refusals, transport
loss and a refusal after a known run remain uncertain. Broader refusal/retry rules land in V08.
