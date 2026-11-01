# Shared interface handoff: Wafflery integration

Status: ready for owner review; proposed additions, not an approved wire design. No sibling
repository has been edited. Implement in the appropriate versioned contract/host interface;
do not add undeclared fields to v0.1 objects.

## Request and evidence

Wafflery is a generic local client with schema-driven task/operation forms, run inspection,
history, and reproducible exports. It must not decode Rascal/Squint private formats or recreate
their orchestration. [Acceptance criteria](../../ACCEPTANCE.md) define the product tests.

Rechecked 2026-10-31: HEAD `8e4d0c3`, published v0.1.0 remains the binding schema; v0.2 is
proposed in PROPOSALS.md. Both tools now have conformance implementations and fixtures. See the
[current source inventory](../integration-sources.md) rather than the earlier snapshots.

Wafflery now supplies concrete [H1](H1-launcher-proposal.md) and [C4 v0.2](C4-v0.2-proposal.md)
proposals, plus a [PROPOSALS.md patch](table-manners-v0.2-proposals.patch). These need owner review,
not guessed host adapters. Rascal M8 is merged; Squint's server is locally merged, not m7-tagged.

## C1 — Discovery, intent, readiness, and estimates

**Existing:** contract sections 3, 4.5, 5, and 11.2 already provide form schemas, named MCP actions,
cost metadata, image slots, and an estimate obligation. Live outgoing calls can be recorded now.

**Needed:** a generic way to distinguish complete tasks, independently callable operations, and
utility actions; group them without inspecting tool names; and explain readiness for the selected
inputs/configuration. Define how configured/default/selectable provider/model choices and effective
output intent are exposed. Credentials present, credentials valid, and action ready are distinct.

Specify an unambiguous association between an action's arguments and its estimate request/result,
including worst case, remaining session budget, and effective cap. Bind estimates to all relevant
input/configuration context. Do not treat provider account credit as tool session budget. Define
unconfigured bootstrap discovery so a missing config does not masquerade as a missing executable.
Current Squint estimates are serialized and create zero-spend run resources through finished().
Record that behavior as known; agree generic control semantics across both tools.
The host treats estimates as control requests, not user history entries, and does not request
draft estimates while that tool is executing. A config recheck must not silently reset budgets.

**Deliver:** minimal versioned types/schemas or documented existing mechanisms, prose semantics,
and paired Rascal/Squint fixtures for ready, partial, unavailable, and changed-configuration states.

**Accept:** a differently named generic client can list tasks versus operations, identify a
mask-only versus painting intent, render required slots/choices, show an actionable unmet
prerequisite, and request the correct estimate without parsing tool-private namespaces. Changing
inputs invalidates the estimate; the tool rejects over-budget calls before paid work. AC02–08.

## C2 — Invocation snapshots and portable dependencies

**Existing:** MCP action names/arguments can be recorded. `input_identity` identifies resolved
inputs but is not a reversible record of them. Rendered mask locations in section 9.2 are not
equivalent to a native Mask needed by some Rascal operations.

**Needed:** tool-provided CLI mapping and effective configuration/provenance; supported persistent
artifact references with type/frame/identity; and an explicit way to export/import or reconstruct
the dependencies needed to repeat an independent operation. Decide what belongs in a shared
description, result, resource, or export action. Don't require one speculative universal SDK.

**Confirmed preservation scope (Q15):** Wafflery retains declared outputs and debugging
artifacts, including intermediate masks, geometry, findings, and report dependencies. Define
how the host obtains durable assets without relying on disposable tool files or guessed layouts.
Exclude caches/model downloads and declare unsupported preservation explicitly.
The host can already save v0.1 results/journals/output images and location data. Keep that evidence
category distinct from full native/report preservation. A location's mask fingerprint has no
locator by itself: ensure referenced bytes are obtainable through declared input/output/resource
bindings, or expose the unresolved dependency. Do not require a client to search a workspace.

**Deliver:** a concrete example for a stateless image operation and one for a saved native Mask;
preserve input roles/fingerprints, relevant versions, and declared provider/model/seed metadata.
Specify unsupported/nonportable states. Export environment-variable names, never secret values.

**Accept:** copy/export a selected saved attempt after editing the form and get the original
invocation. Run a supported bundle from another directory with the old workspace absent and
declared tools/providers available. Required inputs are included by default; reference-only is
explicit. An unexportable native Mask is labeled workspace-dependent, never replaced by a PNG.
Use offline fixtures for reproduction. AC05, AC11, AC13, AC17–18.

## C3 — Tool report address, framing, and lifetime

**Existing:** `renders` gives a command with `{run.dir}`. Rascal's `dash` prints a bare loopback
URL; Squint's `report view` prints a human sentence containing a URL. Neither format constitutes
a common readiness/lifetime protocol in v0.1.

**Needed:** agree machine-readable readiness/address reporting, process ownership/lifetime,
startup failure/timeout, and framing permission for the local host. Identify document-producing
commands separately from long-running HTTP viewers. Preserve the tool's detailed page; the host
must not parse human stdout or recreate it. Local runs with usable directories are sufficient
initially; network transports with null `run.dir` are explicitly deferred.

**Deliver:** shared semantics and a conforming fake viewer, then implementations in both tools.

**Accept:** start at port 0; obtain a ready loopback address; embed the report; close/reopen its
drawer without orphan processes; handle startup failure without marking the run failed. Decide
how reports survive moving/pruning tool-owned assets before promising durable report replay.
AC16, AC19; output/report ownership remains gate G1.

## C4 — Budget/session identity and recovery

**Existing:** per-call cap, per-server-session budget, serial default, cancellation, and resources
are contract requirements. Daily/monthly durable spending ledgers are out of v0.1 scope.

**Needed:** identify session boundaries and explain remaining-budget changes to a host that may
stay open all day. Define behavior across provider/config changes, explicit restart, child death,
and outstanding paid requests. Do not restart a server to silently replenish spending allowance.
Distinguish reconnection to a live session from an interrupted process; no blind resubmission of
an uncertain paid call. Wafflery's first release uses minimal crash handling (Q17): restore
saved history/evidence, flag interrupted attempts with unknown outcome/spend where needed, and
require explicit retry as a new attempt. Durable job recovery or exhaustive crash reconciliation
is not requested.

**Attempt association:** the host sends and records a unique progress token/request mapping.
Specify the machine-readable representation by which the first progress notification names
the run resource; v0.1 prose states that it does, but does not give a wire field. Define behavior
when cancellation happens before that notification. A correlated final result can also bind
the run. Until a reference is known, the host retains unknown outcome and never matches recent
runs heuristically. Consider client correlation/echo only if needed; do not invent a v0.1 field.
Test pre-run JSON-RPC rejection separately from transport uncertainty and actual tool failures.

**Deliver:** lifecycle semantics and fixtures covering session exhaustion, cancellation in flight,
disconnect/reconnect, and server restart. A daily budget or universal provider-balance API is not
requested. Wafflery owns tab/host lifecycle; tools remain the spending enforcement authority.

**Accept:** multiple browser clients cannot authorize work against stale shared remaining budget;
closing a tab does not cancel; cancellation stays pending until the terminal resource is known;
reported spend/partial outputs survive. Restart boundaries are explicit. AC08–10, AC22.

## H1 — Shared launcher/bootstrap agreement (host-owned)

This is a coordination task between Wafflery and both tools; it need not become a new core
contract schema. Agree executable discovery, the `--tool` context, installation source/confirmation,
compatible host reuse, loopback browser opening, and failures. The browser starts after the host
has been installed and launched. There is no host-installation screen inside the host.
Keep tool launchers thin: locate/install the Wafflery binary and invoke it with the agreed
tool-selection argv. Only Wafflery owns locks, attachment, session authentication, bootstrap
secrets, and browser opening. Those details must not be independently implemented in each tool.

Rascal task 8.7 is implemented with `PLAYGROUND`/`playground` lookup and guidance + exit 69,
without installation. Squint task 7.9 is H1-blocked and plans a terminal installation offer. Align them with the reviewed yes → install → launch journey. Noninteractive
execution must not hang waiting for input. Test installation using a fake installer, not downloads.
Agree config/workspace discovery explicitly; don't invent additional launcher flags here. AC01–02/10.

The [host engineering draft](../../ENGINEERING.md) proposes a private managed workspace/run
area per configured tool, outside the relocatable history/asset library. Define a declarative
executable/argv/configuration interface for
passing those locations at server startup. Preserve the host's lack of tool-specific code;
do not infer flags from tool names or silently reuse a user's unrelated workspace. Address
native path dependencies when the library moves. This is an H1 proposal, not a v0.1 schema field.

## Suggested dependency order

Review H1 now so both tool launchers can align. Review C4 for inclusion before the v0.2 tag;
spend implementations now exist. Then review C1 with R3's typed-operation example and assign an
actual R3 milestone; C2 depends on that binding. C3 can proceed independently. Installer release
availability is later than the launcher ABI and does not prevent fake executable tests. The fake host/client workflow
can proceed before either real tool finishes conformance. Return agreed schema/fixture locations,
version, owner, remaining exceptions, and verification evidence to Wafflery.

## Sources checked

- [Contract](../../../table-manners/CONTRACT.md): sections 3–5, 9.2, 10–11.
- [Rascal description](../../../rascal/crates/rascal-mask/src/describe.rs) and
  [contract notes](../../../rascal/docs/contract.md): merged M8 implementation at the current audited revision.
- [Rascal viewer](../../../rascal/crates/rascal-mask/src/dash.rs): `serve` address output.
- [Squint description](../integration-sources.md#squint): `description`.
- [Squint viewer](../integration-sources.md#squint): `view` address output.
- [Rascal plan](../../../rascal/PLAN.md): 8.6–8.8; [Squint plan](../integration-sources.md#squint): 7.5–7.10.
