# Wafflery engineering draft

Status: implemented macOS host design through M5.21, updated 2026-11-03. Product decisions live in
[SPEC.md](SPEC.md), observable outcomes in [ACCEPTANCE.md](ACCEPTANCE.md), and scoped implementation
history in [PLAN.md](PLAN.md). The live pin is published table-manners `=0.4.0`; protocol is
`2025-11-25`. Older version sections below retain the reasoning behind earlier milestones.
[The M5 review](docs/reviews/2026-11-03-m5-operations.md) records current acceptance and limitations.

The host embeds SQLite and prebuilt Vite/React assets; end users need no frontend runtime.
[Foundation runtime](docs/design/foundation-runtime.md) documents the build and browser boundary.

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

Image binding follows each declared slot's cardinality: scalar slots become objects and
`many: true` slots become ordered, non-empty arrays. Omitted optional slots stay absent.
Each occurrence gets a separate execution copy and its own fingerprint, coordinates and role,
including repeated asset IDs. Before copying, the host checks every binding and metadata size:
at most 32 images, 128 MiB per file and 512 MiB total per attempt. Byte reads still verify the
stored fingerprint. Cleanup validates all scalar/array paths before removing owned copies and
never recursively removes unrelated tool files. Completed results must account for all submitted
images per slot, matching role, frame, identity and order. Non-completed results may report an
ordered subsequence; missing occurrences produce preservation notices while retaining the tool
outcome. Duplicate identities are counted without inferring which indistinguishable occurrence
was reported. Images-profile results always require images and locations containers.

The list editor carries coordinates on each selected occurrence, including repeated asset IDs.
Reordering/removal moves the entire selection; examples and history restore saved inputs in order.
A pending import appends to the current selection after checking capacity again. Replacing the
editor/draft invalidates that import's UI callback; already imported assets remain in the library.
The input pane keeps its own scrolling and fixed Run footer on desktop. Optional empty slots are
omitted from both image bindings and coordinate arrays before submission.

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
tool-rendered reports can require reinstalling a compatible tool; explain that locally. Rendering
is a live operation and requires saved declarations/results for the current exact contract. Older
saved evidence remains readable/exportable without a compatibility renderer or invented invocation.

Resolved CLI handoffs use a minimal launch snapshot saved before dispatch, plus the tool's immutable
invocation. Unknown launch argument values and environment values are excluded. Tool configuration
is never read, even for hashing. Owned image bytes are verified before adding local locators; mixed
lists use recorded argument indices, and previews cannot replace inputs. The command uses the saved
working directory/config path and explicit input grants, and is withheld for unresolved store flags,
missing launch context or missing/corrupt owned bytes. It depends on that local store and matching
configuration; portable invocation bundles are a separate export. Copied commands are never executed
by the host.

## Deletion and changing storage location

Keep runs/examples until explicit deletion. Compute references within the host before cleanup;
do not rely on a cached count that concurrent example creation can invalidate. Active attempts
and exports hold temporary references. Block deleting an active attempt until it terminates.

Deletion first removes selected records/references transactionally, then removes unreferenced
owned assets. Recheck references under the host's storage lock before unlinking. Failed deletion
can leave reclaimable files; it must not break surviving runs/examples. Baselines do not prevent
an explicit run deletion; the UI explains that the pin will clear. Examples retain their assets.

Schema 6 stores opaque native files under `Library/blobs/sha256`, with occurrence links owned
independently by attempts, examples and bundles. Schema 8 adds saved invocation bundle ownership. Schema 7 adds frozen input-artifact
occurrences and explicit workspace-dependence labels to example records, with the normal database
backup before adoption. Historical examples lacking native input occurrences cannot claim portable
restoration. Example-origin lineage validates saved port occurrences independently of deleted runs. Publication and reads verify the digest;
cleanup includes only blobs with no surviving owner. Native files are not decoded by the host.

Artifact export/import have a separate two-slot registry and never take a paid-run permit or
create a run. An HTTP waiter has a 10-second deadline; its owned worker retains any staged file
until the RPC finishes or the child is confirmed stopped. Cleanup, restart and shutdown account
for these workers. Native publication and attachment share a storage lease. An import may have
completed remotely after acknowledgement is lost; an explicit retry reconciles by identity.

Automatic native preservation uses two queue-owned workers and at most sixteen waiting attempts.
Duplicate references share verified bytes; repeated occurrences retain separate ownership links.
A failed export or a full queue changes preservation status only. A new host marks unfinished
preservation failed and waits for explicit retry, without replaying the action. Queue workers have
no browser-wait deadline; their transfers retain staging until a reply or confirmed child exit.

Artifact selection checks the receiving registration's resource. Missing resources offer explicit
import of owned native bytes, followed by a validated resource read before the draft is filled.

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
The library UUID and content digests remain stable. Native tool-store relocation is verified against the approved Rascal snapshot; general library relocation/distribution retains its own gates.

## Host and tool processes

On macOS, every host-owned process spawn uses `process::spawn` or `process::spawn_std`.
Their shared mutex covers synchronous pipe creation and spawning only; no wait or asynchronous
work holds it. This prevents concurrent children inheriting temporary pipe ends before close-on-exec
is set, which otherwise keeps another tool's stdin alive after its owner closes it. Tool, report,
observer, launcher and browser-opening paths all participate. See the
[spawn-isolation regression](docs/reviews/2026-11-02-spawn-isolation.md).

Tool `playground` subcommands only locate/install and invoke the Wafflery executable with
the agreed tool-selection context. They do not read locks, authenticate a host endpoint, mint
browser secrets, or open the final browser URL. The Wafflery binary owns all those details.

The Wafflery launcher resolves the configured library and tries to attach to the existing host. The
host owns an OS lock for its entire lifetime and explicitly unlocks on release (including
failed acquisition cleanup), so inherited descriptors cannot extend ownership. A stale discovery
record alone cannot authorize attachment or killing a PID. Authenticate the live endpoint and check host protocol/library
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

V09a separates exact arguments and materialized input copies into an owned preparation before
saving an attempt. Preparation creates no history record; persistence transfers the same
`execution/<id>` paths without rereading originals or library bytes. A storage read guard prevents
cleanup from deleting asset references before that transaction commits. Failed or abandoned
unclaimed preparations remove only their private temporary directory. Submission is an owned
task, so dropping an HTTP requester cannot interrupt materialization, persistence or dispatch;
an uncertain reply is retried with the same submission id. Explicit cancellation still uses the
attempt cancellation control. V09b retains up to 16 preparations and 1 GiB of copied image bytes per workspace, with a
15-minute absolute monotonic expiry. Identical preparation requests return the same deadline;
conflicting inputs cannot replace an identity. Submission checks and consumes a live matching
process session, tool session and declaration under the same discovery guard as dispatch.
Preparing consumes no execution slot; V12c enables paid prepared submission through the quote guard.
V12b consumes request identity, session/declaration and quote eligibility atomically. Paid drafts
require their current non-invalidated estimate ticket, an allowed unexpired quote attributed to
the tool/action/session and positive cap, and no outstanding reader. Consumption returns that
quote with the owned copies, so disposal/re-estimation cannot reuse them. Before paid persistence,
the host rechecks each materialized copy's bounded size and SHA-256 without following symlinks
or accepting hard links. Empty optional scalar slots remain omitted; every list occurrence is
checked. The tool must still check the binding as the call arrives. Selected binding, cap, worst
case, model/config identity, expiry and attribution are committed in `observations.estimate`
with the attempt and input references, without arbitrary effective/debug extensions. A rejected
or changed copy creates no attempt and its consumed preparation cannot be reused.
The submission lock serializes capacity preflight through copying, so in-progress preparations
count against the bound before any bytes are copied. Duplicate image bindings count per copy.
A weak-owner sweep frees expired copies every 30 seconds; every lookup/consume also checks expiry.
Restart, declaration invalidation and close release only unclaimed drafts. Library cleanup must
go through Coordinator: it checks retained drafts before requesting the storage write lock and
refuses while they exist, preventing a queued writer from blocking later asset imports. The
receipt's wall-clock deadline is for display; monotonic time controls eligibility. Authenticated
HTTP controls now expose prepare, receipt lookup, discard and explicit prepared submission
through the existing same-origin bearer boundary. The path id must match the strict saved request;
other workspaces cannot resolve these ids. None of the preparation controls create history or
call the action until explicit submission. V11 connects these controls to an explicitly requested browser estimate; V12 connects guarded paid dispatch.
Before copying, the host writes a private versioned ownership marker. Startup validates its
execution root, skips recorded attempts before inspecting their contents, and reclaims only
marked, unclaimed directories containing recognized regular input copies. Unknown content,
unmarked directories, hard links and symlinks are preserved; cleanup never follows symlinks or
recursively removes discovered directories. A crash between persistence and marker deletion
cannot expose a saved attempt to this cleanup. A retained marker is also removed by terminal
input cleanup after the tool has relinquished those paths.

v0.3 estimates (PLAN V10–V11) run outside the host execution reservation.
Paid dispatch is enforced by V12; V13 adds specific outcome guidance. Estimates remain responsive while
an action runs; they neither consume a run-dispatch permit nor create attempts. The exact allowed
quote, positive cap and binding are checked again under the host's execution reservation before
dispatch. Tool-side caps and budgets remain authoritative; a quote is not a reservation.
Keep one active execution per tool and reject a second submission as busy. The transport now
exempts exactly reserved `describe` and `estimate` calls from its persisted execution permit;
controls cannot consume a staged action permit, record attempt wire evidence or reset its
progress association. All other tool calls still require matching action authorization. V12a adds a validated estimate
slot to that permit. An unpriced permit rejects any estimate metadata; a priced permit requires
its exact opaque binding, allowed decision, matching action and live expiry. Expiry is checked
again after durable wire evidence is recorded and before the transport write is polled. A denied
write can therefore retain its wire intent as evidence without claiming bytes were sent. The
coordinator now accepts paid work only through owned prepared submission; ordinary submissions
cannot authorize a new paid attempt.
V10b1 implements the estimate-validation boundary: strict structuredContent/schema, tool/action/
serving-session attribution, shared ledger consistency, expiry and cap/decision checks. Valid
readiness refusals retain unknown prices and configuration; they never authorize dispatch. The
opaque binding remains server-side evidence. The independent peer and contradictory fixtures
exercise this boundary. V10b2 adds the owned Rust estimate service. V10b3 provides authenticated
POST/GET `/api/preparations/{id}/estimate` controls: POST accepts only an empty object, and
GET never renews either lifetime. Display responses omit opaque bindings, input paths and raw
effective extensions. Bindings remain with the retained preparation for the paid-dispatch gate.
Accepted re-estimation clears the old quote; invalid or busy requests leave it unchanged. A
per-request ticket prevents a reported timeout from leaving a saved quote accessible or removing
a newer quote. Both UTC expiry and elapsed monotonic lifetime apply. Workspace snapshots show
the latest validated ledger observation without changing the declaration revision. A separate
short-lived cache lock preserves that observation while an estimate holds the request mutex;
starting another estimate cannot display an older startup balance. Display amounts never authorize
spending. At most one estimate reader runs per tool, outside its execution permit.
The 30-second deadline includes ledger-lock acquisition. A timed-out dispatched request keeps
its input lease and estimate slot until its original reply completes or child exit is confirmed;
there is no automatic retry. The ledger mutex is released at the deadline, and a late reply
cannot replace newer observations. Requester cancellation cannot abandon this worker.
Discarded/expired drafts become unavailable immediately, but leased copies still count against
retention limits and block library cleanup until the reader ends. Shutdown closes owned children
before joining these workers. A returned quote must still match the same prepared object,
current declaration and owned client. Browser estimate controls are connected in V11; paid API authorization is implemented in V12c.
Controls do not wait for
an execution slot. The former Squint behavior that created estimate runs describes v0.2 integration
history only: v0.3 prohibits it. Paid Run uses the owned quoted preparation; the API requires its exact validated binding.
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

Registrations separate `allow_dirs` (caller input permissions) from `evidence_dirs` (host reads of
returned images and run/report directories). Both are required arrays of existing absolute
canonical directories. The serving executable receives repeated literal `--allow-dir` flags from
the input list; registration arguments cannot supply those flags themselves. Only the dedicated
`Library/execution` materialization/import area is added automatically. The asset store, library
parent, tool workspace and unrelated home/project folders are not implicit grants. Output paths
are checked against `evidence_dirs`, never input permissions; exports will use this same boundary
when their controls land. Document-render output retains its existing exact-file permission.
Resource/inline results retain size limits and fingerprints.

MCP roots are neither advertised nor served. Input grants are enforced by the tool, whose config
may narrow them; they do not sandbox a locally installed executable. Tool binaries remain trusted
local programs. Avoid claiming OS-level isolation that is not implemented.
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

Live execution accepts exactly published table-manners =0.4.0 from crates.io. Older saved records remain readable without migration or replay. Paid
actions must declare a positive max_spend_usd and expose the reserved estimate MCP control,
which is excluded from the Actions catalogue. Paid dispatch requires the owned priced preparation.
List slots preserve input order and frames. M5.02 replaced MCP roots with explicit launch grants
while retaining protocol 2025-11-25. M5.03 pins v0.4 and checks description relationships after
structural validation: declared groups/types, owned type namespaces, unique actions and ports,
and direct parameter/input collisions. This applies at the shared live contract boundary, including
discovery and renderer resolution. Composed/referenced parameter checks, callable-schema
relationships, remaining staged validators and effective-grant display wiring follow separately.
Session resource reads and estimate-embedded sessions now check effective directories against that
owned child's canonical requested input grants before publishing an observation. Effective access
may narrow or deny all, but must remain unchanged within the session. Rejected observations leave
the prior ledger intact; an explicit replacement starts a new client and observation history.
Filesystem checks apply to the first observation; subsequent unchanged grants remain valid if a
directory disappears or is replaced, so the ledger stays observable. A replacement client validates
its directories afresh. The tool still enforces access on each actual read.

First-release history records Wafflery-started attempts. Tool run resources support reading
those attempts, not heuristic adoption of unrelated runs. Automatic import of prior CLI/agent
history is deferred; a later explicit import must label unavailable dependencies honestly.

## Verification before claiming these behaviors

Browser host fixtures capture process closure when spawning, establish their teardown credential
while the readiness handshake is valid, and share one idempotent stop lifecycle. Shutdown must
not bootstrap through disappearing IPC or infer exit from a missing socket. Transport failure
during teardown is accepted only after a clean process close; force still requires the host's
waiting phase. Remove temporary roots only after closure. Assertion timeouts and retries are
unchanged; see `docs/reviews/2026-11-01-host-lifetime-ci.md` for the hosted trace and regression.

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

- C1/C2/R3 host integration is verified by M5.21 within declared capability/portability. C3 report
  framing/lifetime is verified, but reconstructing arbitrary tool-owned report dependencies remains
  unsupported. Unportable artifact types retain workspace dependencies honestly.
- H1 uses canonical `wafflery` and `WAFFLERY_BIN`, with compatible-host attachment. Automatic tool
  discovery, a real installation channel and release packaging remain separate M4/M7 work.
- Preserve tool-owned session spending and explicit restart boundaries; durable job recovery and
  a cross-restart spending service are outside this exploratory host.
- Establish release macOS minimum/architectures and installer verification before M7. Native Safari
  manual evidence remains separate from Chromium/WebKit automation.
- Evaluations and agent orchestration require later approved plans. V20 adds import feedback and
  bounded thumbnails; it does not silently resize source images or infer undeclared tool limits.

### v0.3 readiness controls

`POST /api/tools/{id}/refresh` is an authenticated same-origin control. It rereads description
and session without taking the execution permit, updates only live discovery state, and emits a
workspace update. A per-tool discovery mutex serializes reads through publication; submission
holds that same owned guard through preflight and the actual transport write, then releases it
before waiting for the tool result. A concurrent refresh cannot replace authorization between
validation and dispatch. Saved attempt declarations stay immutable. UI drafts survive refresh.
A definitive pre-run RPC error is a terminal rejected attempt. Plain RPC errors and validated
-32602 `not_ready`/`invalid_arguments` refusals preserve raw evidence, an explicit zero-spend
observation and setup requirements where present, without fabricating a tool run/result. Local
execution inputs and the reservation are released after preservation. A repeated HTTP submission
still returns its original attempt; an explicit corrected attempt uses a fresh id. Malformed or
unrecognized structured refusals, transport loss, reused claims and errors after a known run
remain uncertain. A schema-valid reused-id reference is checked against the original saved
attempt's tool, session and client id before resource recovery. Any RPC error after a known run
uses the same bounded recovery. Raw errors remain saved alongside recovered evidence; null reuse
references never release the claim. No recovery path dispatches another action. Progress
association is captured in transport receive order before rmcp moves metadata into notification
context or schedules handlers; handler progress remains display-only.

Definitive refusals display “No run created” and the tool's reason instead of a comparison or
missing-result warning. The browser retains submission identity only while its acknowledgement
is uncertain. Once submit returns an attempt, its cached request is cleared before any history
refresh; a refresh failure cannot make the next deliberate run reuse an acknowledged attempt.


The browser retains one prepared draft for its exact action/declaration/session, parameters,
cap, ordered image bindings and coordinates. Any input edit, including malformed JSON, invalidates
its display and discards its preparation. A late reply cannot restore that quote. Preparation
and quote expiry both apply; hidden-tab visibility restoration rechecks the display deadline.
There are no automatic estimates or paid calls. Session amount observations update the balance
without changing draft identity; the tool remains authoritative at execution time. The dedicated
cap editor is excluded from generic parameter fields and whole-object JSON edits cannot replace
it. Abandoned-copy disposal is best effort; the host's bounded retention remains authoritative.


Paid prepared submission consumes exactly one owned priced call and retains the selected binding
in both the private transport permit and `_meta["table-manners/estimate"]`. Correlation/progress
metadata remain alongside it. Direct ordinary submissions, caller-provided bindings, changed
requests/copies, stale declarations/sessions and expired/refused quotes cannot authorize paid work.
An acknowledged duplicate returns the same saved attempt without another dispatch; a new attempt
needs new preparation and estimation. Selected authorization facts remain in evidence archives.
The independent paid peer reports synthetic spend only; real-peer/provider validation remains V18.


Browser paid submission transfers its immutable request out of draft ownership before sending.
Edits and quote timers cannot dispose inputs while that request is in flight. Acknowledgement
must match both the attempt ID and the complete request (object-key order is insignificant;
image array order is significant). Network/server uncertainty retains that identity for explicit
same-ID retry, independently of the currently selected tool declaration. Definitive host input/
conflict refusals discard the preparation; a new attempt requires a new explicit estimate and Run.
No caller-supplied binding is sent, and neither a quote nor submission is retried automatically.

After preserving a `credential` outcome or definitive `not_ready` refusal, the host refreshes
that tool's description once before releasing its execution reservation. Other tabs receive the
usual invalidation; changed readiness invalidates retained estimates. This is discovery only,
never an automatic retry, configuration change or session restart. A failed refresh leaves setup
unavailable without rewriting the saved run or dropping its spend evidence.


The selected-run outcome summary derives failure guidance and spend from saved evidence, never
from the edited draft. Missing, malformed or unfinished spend remains unavailable/not final;
only a definitive no-run observation can show zero without a result. Unresolved totals are upper
bounds, rounded upward at display precision. Estimated settled spend identifies configured prices.
Recovery guidance never retries, increases caps or resets sessions. The setup drawer can bind to
the saved request's tool/action; refresh and confirmed restart use that context, while draft
inputs retain their own selected tool/action and quote lifecycle.

### Declared report command boundary

V14a resolves report commands from the selected saved attempt's description and result. It
requires one matching action/render, matching tool/action evidence, and an existing absolute
run directory within the serving client's canonical `evidence_dirs`. A null, missing or
unauthorized directory has no view; the host never guesses a replacement. An explicitly selected
copied directory can be rendered only within those directories. Canonical registration paths may not
silently follow a replaced symlink. The resolver retains an open directory handle and rechecks
its device/inode and canonical path immediately before building the command.

The first argv element must be the declared tool name and is replaced with the serving client's
exact executable. Remaining arguments stay literal: only whole `{run.dir}` and server
`{frame_ancestor}` placeholders are expanded. Embedded/unknown braces and NULs are refused.
Every view binds its run directory; servers also bind the host's exact HTTP loopback origin.
Commands are limited to 128 arguments and 128 KiB before and after expansion. Server readiness
limits remain the contract's 1–60 seconds. Documents have no readiness field.

Command construction uses the existing minimal base environment, with no credential/env
requirements, API bearer or bootstrap token forwarded. It performs no shell expansion and does
not spawn. Process ownership, output bounds and authenticated viewer routes follow in V14b–V17;
reports remain unavailable in the UI at this slice. These path/identity checks authorize host
operations; they do not sandbox the installed tool against filesystem changes after launch.

V14b1 adds an owned render worker beneath that boundary. It retains the child while stdout
and stderr are drained concurrently, capped at 8 KiB and 64 KiB respectively. Overflow/read
errors request graceful stdin closure; truncation is explicit in the observation. The worker
preserves exit codes and bounded diagnostics. Closing or dropping the last request handle
closes stdin. V15c2 extends this to a two-second EOF grace followed by SIGTERM to the owned
group and another two-second grace. A group that still runs remains owned in `NeedsForce`;
only confirmed force sends SIGKILL to that pinned group. Force acknowledgement is a
separate control reply, not an inferred transient status. A wait error never proves exit.

The low-level process layer has no host route. V14b2 retains handles in the coordinator through
request cancellation and shutdown. Report content remains gated on isolation and framing.
Synthetic renderer fixtures exercise process behavior without calling a provider.

V14b2 retains report handles in the coordinator before replying to an open request. Each saved
attempt has at most one live view; reopening the same running view returns its existing ID,
while changing views requires confirmed exit. The host retains at most 16 view slots, evicting
only confirmed exits when capacity is needed. Requests cannot supply executable paths, argv,
run directories or framing origins. The current serving tool name must match saved evidence;
resolution uses that serving client's canonical descriptor. A replaced registration cannot
silently reinterpret an old tool's report command.

Authenticated routes are `POST /api/attempts/{id}/views` with `{name}`, `GET /api/views/{id}`,
`POST /api/views/{id}/close` with `{}`, and `POST /api/views/{id}/force-close` with
`{confirmed:true}` after graceful close has timed out. Responses expose bounded stderr and
process state, never raw stdout or report content. There is no browser report control yet.
Quit closes all retained views concurrently and waits for confirmed exit; explicit host force
quit also covers waiting views. A failed report never rewrites its saved attempt, status or spend.

V15a adds the server readiness validator: one complete stdout URL line, literal HTTP loopback
with an explicit nonzero port, and an origin distinct from the workspace. It rejects parser
normalization shortcuts, credentials, extra stdout, broken output and stopped processes.
Startup deadlines are whole seconds from 1 through 60; a failed view cannot recover on late
output. A ready view remains subject to output/process validation after its startup deadline.
V15b connects the validator to retained server views. A watch-driven worker enforces the
original startup deadline without browser polling, continues checking output after readiness,
and closes stdin on failure. Process state retains explicit close intent so normal viewer close
cannot become a startup error when notifications coalesce. Failed supervision preserves its
diagnostic even after the child exits; saved run evidence is untouched. Independent HTTP
fixtures exercise actual listening, exact frame-ancestor response policy and EOF shutdown.
Report addresses and content remain unavailable to the browser until the isolation/framing gates.

V15c1 introduces the macOS group owner used by V15c2's lifecycle worker. Each child is
started in its own process group. The owner observes leader exit with kqueue without reaping
it, then checks bounded `/bin/ps` output for that exact group. It requires the pinned leader
to be present and every member to be a zombie before reaping; stopped or unknown members are
not drained. Darwin's `H` and `?` Mach states, including the observed `?E` exit transition,
are valid but not drained. A received NOTE_EXIT remains known even if a later independent
membership snapshot is uncertain; a one-shot event cannot be recovered after being erased.
Empty/missing-member or malformed observations remain errors, never proof of exit.
Failure or a registration race preserves ownership. Only this owner can signal
the pinned group, and signaling is disabled after reap. The observation helper uses a minimal
environment, bounded output and timeout; an independent worker kills/reaps the helper even
if the caller cancels its wait. This uses the existing pinned nix crate's event/signal features.

These process groups manage cooperative descendants; they are not an OS sandbox and cannot
contain a program that deliberately changes its process group or session. V15c2 polls the
nonblocking exit event every 100 ms; only exited/unwatchable leaders need a membership helper.
An early leader exit with surviving descendants starts SIGTERM cleanup and keeps the slot
until the group drains, including after explicit force. Failed observations retain the group.
Report processes currently require macOS; unsupported platforms refuse before spawning.
V15d attaches a five-minute in-memory idle deadline to each view. Opening the same active view
or authenticated `POST /api/views/{id}/activity` with `{}` renews it; state reads do not. Activity
after the deadline cannot resurrect the view, even if the timer has not yet run. Expiry closes
the process through the same owned EOF/SIGTERM lifecycle, without automatic force, and exposes
`idle_expired` for a later UI explanation. Closed/replaced IDs reject activity. There is no
durable lease or caller-supplied TTL. V17b will send activity only during visible inspection.

V16a reads one document output only after confirmed exit zero and complete bounded stdout/
stderr. Stdout must contain one absolute UTF-8 path line. The grant pins the original run
directory, rejects symlink/non-regular output and checks ancestor device/inode identities to
exclude outputs inside the run even through case/path aliases. Before/open/after snapshots
reject replacement or mutation; the source file is neither deleted nor served in place.

Each immutable copy is capped at 32 MiB, with a five-second async read deadline. At most 16
blocking readers may remain outstanding globally. A permit lives inside the blocking task,
so caller cancellation or timeout cannot free capacity before that reader finishes; excess
reads are refused without queuing. Only the declared file is read, plus ancestor metadata for
containment checks. Serving/isolation remains V16b and retained view integration remains V16c.

V16b serves each immutable copy from a separate literal-loopback port and an unguessable
document path. There is no filesystem router, neighboring-file grant, API bearer, cookie or
CORS grant. Exact Host validation rejects DNS aliases and duplicate Host headers. Unknown
paths, query strings and absolute-form targets are refused. Known HTML/SVG/image/JSON suffixes
receive their explicit media type; other names are shown as plain text with sniffing disabled.
All responses use no-store and no-referrer. Content CSP allows inline script/style and embedded
image/media/font data, denies network fetches and nested content, sandboxes active content, and
permits framing by only the exact workspace origin.

A scoped, bounded CSP-report endpoint converts blocked-resource reports into a generic content
error. It does not retain blocked URLs, snippets or policy payloads. After failure, the document
URL is unavailable and further content requests receive 422; the run remains untouched.
Browser enforcement and the nested-frame integration are verified in V17, rather than inferred
solely from these HTTP boundary tests. No content is embedded yet.

The listener owns at most eight HTTP/1 connections, each with an eight-KiB header bound,
sixteen-KiB violation-body bound and ten-second total connection deadline. Close or loss of all
owners stops accepting, aborts and joins every connection, then confirms closure. Hyper and
hyper-util are direct dependencies at their already-locked versions so this boundary can own
each connection instead of relying on detached server connection tasks. Immutable bytes are
shared across responses without copying the entire document per request. V16c connects this
server to the view's idle/shutdown lifecycle.

V16c1's document monitor retains generation, immutable reading and content serving as one
owned lifetime. Generation must finish within thirty seconds, then the independent five-second
reader bound applies. A renderer's exit does not close a ready document. A document idle lease
remains active through content serving; observation does not renew it. Close, idle expiry,
policy failure or last-owner drop closes the content listener and requests graceful renderer
closure. A stuck process remains owned until an explicit, confirmed force action; a premature
force request is refused without starting closure. A closed monitor means both the content
listener/connections and renderer are confirmed stopped. Pending reads cannot publish after
close; their bounded worker permits remain held as described above. Ready URLs are cleared
when cleanup begins. Coordinator wiring and capacity accounting remain V16c2.

V16c2 connects that monitor to workspace views. A completed document renderer can still have a
ready view; the view occupies a slot until both its content server and renderer are closed.
Reopening the same active view renews its lease and keeps its identity, while replacement waits
for confirmed closure. Activity, close, force-close and workspace shutdown use the full document
lifetime. Public observations expose status and bounded diagnostics, never output paths or URLs.
Output failures preserve the saved run. Deadline checks consult the clock even before the expiry
worker runs, so activity or later viewer grants cannot revive a deadline that already passed.
Reports support V17 framing; visible drawer controls remain V17b.

V17a1 adds host-lifetime frame grants separate from browser API credentials. Each 256-bit random
token names one view identity, lasts sixty seconds and is removed atomically on first use.
At most sixty-four unconsumed grants exist; consumption or expiry releases capacity. Nothing is
persisted or logged. The issuing handler must first validate the active view, and the consuming
wrapper must revalidate it after consuming the grant, even if loading then fails. Grant storage
alone does not authorize content or relax frame headers.


V17a2 issues grants only through authenticated view controls. The wrapper consumes a grant
before revalidating the current view, rejects HEAD/query requests, and never revives a closing,
expired or replaced view. Server output/process state and document content state are checked
again rather than trusting a cached ready observation. Reading a frame does not renew the lease.

The app permits frames only from its own origin. A trusted wrapper contains only host markup,
uses a per-response CSP allowing the selected report's exact separate origin, and frames it with
`allow-scripts allow-same-origin` only. Both ancestors retain the workspace origin. The wrapper
uses no-referrer, no-store and SAMEORIGIN framing; failed wrapper requests retain the ordinary
DENY policy. No API bearer or bootstrap credential enters a frame URL or report request.

Chromium and WebKit load both renderer kinds through this nested boundary. A document's
blocked-resource report closes its view without changing saved evidence. WebKit sends that
no-referrer report POST with `Origin: null`; only the private document-capability endpoint
accepts it, still validating the exact document URI, content type and bounded body. Foreign
and duplicate origins remain refused. Workspace API origin rules are unchanged. V17a3 adds
adversarial navigation/API checks before V17b exposes the drawer controls.

V17a3's independent active-content probes verify blocked parent DOM/storage access, top-level
navigation, popups, form submissions and unauthenticated workspace requests in Chromium/WebKit.
A real second loopback listener receives no request when the report links or redirects there;
the wrapper reports the frame-src violation. This is browser isolation, not an OS sandbox for
the renderer executable. Report-origin behavior remains the tool's responsibility.

V17b1 adds a typed private report client and one workspace-level ownership manager, retained
across drawer mounts. It serializes open/replacement/close, closes late successful opens after
dismissal, and rejects stale status/frame responses. The authenticated attempt-view lookup
observes only: it neither starts a renderer nor renews its lease. Lost opens remain unresolved
until host restart because even a current-view lookup cannot prove a pending POST settled;
lost closes recover only through their retained view identity. Definite pre-spawn refusals permit
a later explicit open, with no automatic retry. Force remains an explicit confirmed action for
a positively observed needs-force state.

V17b2 exposes Reports beside the saved evidence in the shared drawer. The saved declaration
supplies report names/kinds; choosing one does not start it. Open is explicit. The frame uses
only V17a's validated host wrapper, with readiness, diagnostics, inactivity and uncertainty
states kept outside tool content. Force stop requires a separate confirmation focused on Keep.
Switching to evidence, changing reports or closing the drawer unmounts inspection and releases
the owned viewer. While that report tab is visible, status polls every 500 ms and ready-frame
activity renews every 30 seconds; hidden pages send neither. Exited views stop automatic status
polling. Remount resumes observation without opening a renderer; explicit Close keeps observing
the retained view until closure or a force-required state. Startup failures preserve the saved run and its evidence tabs. Storybook covers ready,
starting, failed, expired, uncertain, force-required and unavailable views at shared drawer sizes.


M5.15 assembles invocation bundles from the saved result/invocation, verified original input images
and exact owned native exports. By-value shared locations need no export. Missing native inputs and
image references lacking preserved originals are explicitly workspace-dependent; image previews never
stand in for native bytes. Omissions cover a whole declared slot/port and require a reason. The
manifest retains occurrence order and duplicates while members are deduplicated by their canonical
identity-derived name. Conflicting bytes for one name refuse assembly.

Bundles use the existing tar dependency with fixed uncompressed ustar headers, `invocation.json`
first and other members sorted. MIME suffixes are fixed (png/jpg/json/tar, otherwise bin), never
borrowed from an export path. Limits: 8 MiB manifest, 64 MiB/member, 64 members including the
manifest, 256 MiB including container overhead; safe lowercase basenames are at most 96 bytes.
The temporary file is hashed after completion. Assembly itself makes no tool calls, reads no configuration and creates no completed bundle record.


M5.16 persists completed archives under `Library/bundles/sha256/` with schema 8 manifest/member
metadata, image/native ownership and source-attempt associations in one transaction. Publication
revalidates original inputs under the cleanup write lease. A failed commit rolls back ownership and
removes a newly published file; an existing archive is never overwritten. Downloads hash the exact
owned file before streaming it. HTTP cancellation does not release assembly capacity early; saved
archives survive interrupted downloads, source-run deletion and restart. Bundle lists return bounded,
paged summaries. Cleanup includes bundle usage and retains bundle-owned inputs. Bundles do not expire
and source-run cleanup never implies bundle deletion; dedicated deletion is not implemented yet.

The pipeline's Invocation bundle tab creates an immutable saved review, then downloads it separately.
The workspace Bundles drawer can reopen it after source deletion. Requirements, included original
files, native copies, explicit omissions and unportable inputs precede download. A standard CLI example
is shown only for one tool and empty dependency lists; it uses labelled example paths. No export/import
control or action runs during assembly/review/download, and no config or credential contents are read.

Bundle save briefly holds the exclusive storage lease while revalidating input bytes and hashing/
publishing the archive, so imports and preservation wait during that bounded operation. If cleanup
removed the source after assembly, save reports a conflict before publication. A process crash between
file publication and the metadata transaction can leave an unowned archive file; it is not a completed
bundle, is not counted as a saved record, and can be adopted by an identical later save. Automatic
orphan reclamation is outside the current recovery scope.

M5.17a reads bounded uncompressed ustar into immutable bytes plus checked member offsets. Raw tar
iteration exposes extension records for rejection; safe standard ustar prefix encoding is accepted.
Headers, member order/names/count/size, padding/end blocks and all manifest schemas/references/hashes
are verified before publication or tool contact. Duplicate JSON keys and unsupported versions are
rejected. Image format/dimensions are checked from headers; full raster decoding is not part of
archive validation. Native files are opaque. Occurrence checks use contract manifest/result fields,
never private parameter interpretation. The reader itself has no filesystem or tool side effects.

Validated archive data is immutable through the reader API. Archive IDs hash the exact input bytes;
safe alternate ustar encodings are not normalized or equated. Native import idempotence follows the
artifact identity. Current tool descriptions must establish typed port coverage/readiness before
publication/import: arbitrary uploaded result fields are not authenticated proof that all required
inputs are present. References to unstored earlier outputs require the later chain edge resolver;
this reader currently requires files or explicit dependency records for such inputs.


M5.17b's private upload API keeps at most two uncompressed archives (256 MiB each) in memory with
30-second bounded uploads. Validation has no filesystem/RPC effects. A separate explicit selected-step
import checks the current public description before publication, retains the exact original archive,
and performs only input import controls. Its tracked worker outlives dropped HTTP waiters and drawer
closure; group ownership blocks restart between controls. Per-input progress retains partial success.
Native files remain opaque and use granted file staging because no export MIME type is in the
manifest. The receiver verifies native contents; the host rereads each imported resource. Imports do
not create attempts or authorize paid calls. Uploaded reviews are session-local; explicitly retained
archives and bundle-owned image inputs survive restart. The shared Bundles drawer exposes upload
review and explicit selected-step import with small progress-only polling. Closing it stops browser
observation, not the worker. Reopening finds session-local reviews and their current phases. Active
reviews cannot be dismissed. Current tool setup/requirements, omitted inputs and original invocations
are displayed separately from the file-verification label. Opening a runnable draft follows in M5.18.

Worker registration takes its shutdown-owner lock before spawning: HTTP cancellation cannot leave an
untracked import. Shutdown stops between inputs, preserving any already-running RPC through its
terminal response/confirmed exit and keeping remaining inputs pending. Unknown native export MIME
never enters the shared native blob cache: the retained original archive owns those bytes, while the
receiving tool owns the imported durable artifact. Image staging writes run off the async executor.


M5.18 adds optional `source_bundle` provenance to serialized submissions. Retained bundle identity,
step, tool name and action are checked before normal preparation and dispatch. The attempt and its
bundle association use one SQLite transaction; no schema migration or copied authorization is needed.
Changing parameters/inputs is permitted because this link records origin, not equality. The original
archive/result remain immutable. This metadata never enters the tool's argument object.
The private ID-specific image metadata endpoint returns owned metadata without a raster decode or
depending on the newest-hundred image catalogue.

The import review rereads current progress before its data-only adapter opens a normal workspace
draft. Source images use `/api/assets/{id}/metadata`, with the same private boundary and owned-ID
validation; this avoids the limited image catalogue and performs no raster decode. Native refs come
from confirmed import progress. Task shared locations use the existing typed by-value port controls.
An unresolved imported port blocks both estimating and Run until its current replacement list is
explicitly confirmed, even if optional. Paid drafts clear the archived cap; users choose a new positive
cap before a fresh estimate. Opening does not replace a tool session or reset its spending allowance.
Source linkage appears in the inspector and input pane. Output identity differences are informative,
while ordinary tool outcomes remain authoritative.


M5.19 exports explicitly selected, distinct, ordered attempts (at most 16) through `POST /api/bundles`
with an `attempts` ID list. Assembly shares one storage read lease, unions ownership, deduplicates exact
members, enforces cumulative archive/manifest bounds and preserves canonical single-step bytes. The
write-lease commit rechecks every source before publishing the archive and all source associations.
HTTP cancellation retains the assembly worker and its single shared create permit. No tool controls,
configuration reads, action calls or extra attempts are involved.

`tools::chain_edges` is the common pure resolver for export and incoming review/replay. It matches only
earlier public artifact/image occurrences, preserves port-local item order and global profile indices,
and coalesces an image profile row only via a unique linked preview with identical image bytes and
coordinates. Optional pixel-digest hints do not affect image identity. Native refs retain their opaque
frame and tool ownership; previews never become native data. Shared location values stay literal.
Zero matches denote external input, one a fresh-output edge, and multiple matches refuse the chain.
Export additionally checks all inferred edges against saved host lineage; equal identities cannot
substitute a different selected source. Task image slots may carry existing host lineage metadata.

The private API derives `chain.edges` and `chain.problem` alongside reviews; these fields are neither
stored nor added to the contract manifest. Incoming ambiguous archives stay reviewable as individual
steps. Native exports are included
conservatively for internal edges too; their presence never permits stale-output chain replay.

Connection resolution has a conservative one-million-candidate search budget covering both input
matching and mirror scans. Oversized graphs remain individual-step reviews with an explanation.
Uploaded analysis is computed once with parsing on a blocking worker and cached for the review's
lifetime; saved-record analysis also runs off the async runtime.

Ordered selection and connection review live in the shared drawer. Collapsing selection keeps its
observer mounted until the owned save resolves, so completion releases the drawer controls. The
builder respects the parent busy gate as well as its own work, preventing overlapping save actions.


M5.20a adds a bounded guided replay registry (two live reviews, up to 16 selected steps) and core Rust
methods to stage one step without dispatch. The verified archive is retained, while normal workspace
preparation, cost estimation and Run remain the only action paths. A replay context on source-bundle
metadata carries its review ID/generation, outside tool arguments and the contract manifest. Both
preparation and dispatch check the current staged context and every connected binding/lineage.
Claiming a run or stopping/resuming invalidates only this review's other retained preparations;
ordinary same-tool drafts, recorded spend and session identity remain unchanged.

Fresh output planning normalizes public output profiles once per producer with one total search
budget, on a blocking worker. Stable port/item ordinals select new outputs rather than old identities
or global array positions. Fresh image data requires matching preserved owned bytes; native previews
remain previews. Resolved internal edges become host-only prepared replacements and do not import
old native copies. External inputs still follow the strict archive import path. No host-only input
replacement can be decoded from an uploaded manifest.

Stop invalidates the current generation and prevents further input work; any current ordinary run is
cancelled through its usual owner. Queued staging/import ownership and native preservation must settle
before resume/dismiss can proceed. Completed evidence is reloaded and verified on explicit resume;
missing outputs, changed/missing public ports or an unavailable native resource block the consumer.
Replay reviews are session-local; this is not crash recovery. Historical runs and archives remain
saved. Rollback must keep the optional context parser when removing replay entry points, and must
stop/drain owned work before reverting orchestration.

The private chain API accepts only retained bundle/review IDs and receiving installation IDs. It
acquires a registry slot before reading a saved archive, verifies its bytes again, and retains the
archive independently of its source review. Progress reports whether staging/import/run preservation
is still settling; Stop is available while those owners drain, and resume/dismiss remain blocked.
Connected many-input bindings may be explicitly reordered, but every connected edge must retain a
distinct fresh value and actual producer lineage. Missing external inputs remain explicit. Core
image-slot lineage and operation-port lineage are combined rather than overwriting each other.
Opening history or examples strips the archived live replay context and creates an independent
draft; it does not rejoin or advance a chain.


Guided replay is exposed in the shared invocation-bundle drawer. Starting or preparing a review
never dispatches an action. The current step opens the ordinary workspace with verified fresh inputs;
after its Run is claimed the same context cannot submit another attempt. Continue chain observes the
saved outcome and offers the next step. Stop and resume are explicit, and closing the drawer leaves
owned work running. Leaving a chain stops the review and removes its live context from the draft.
Paid steps clear archived caps and always use current-session preparation, estimate and explicit Run.
The drawer renders loading, changed-output, blocked, settling, stopped and completed states with the
shared controls and visual policy. No new component library or automatic pipeline runner is added.
