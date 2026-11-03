# Squint handoff: generic check workspace

## Current disposition — 2026-11-03

Approved M10 `6dd1e3a` provides the v0.4 task description, resolved invocation, list images,
explicit grants and CLI bundle replay. M5.21 verifies ordered/repeated references, actual path
refusal/granted success, and replay after original inputs/library/run directories are removed.
The estimate control no longer creates a run; that old request is resolved. C1/C2 integration and
C3 server/document framing/lifetime are verified. Report sources remain tool-owned; no full report
reconstruction is claimed. Offline CLI checks use a consistent test-only fake profile/model.

See [integration evidence](../integration-sources.md) and [M5 review](../reviews/2026-11-03-m5-operations.md).

## Historical request packet

The dated status and requests below are retained as historical context; current disposition above
supersedes statements that these agreements are still awaiting a first proposal.

Status: refreshed 2026-10-31. M7 has a local tag and the owner reports it pushed/released.
These are remaining integration requests, not a replacement implementation plan. No sibling files changed.

## Current evidence

HEAD `fcde474`, with active uncommitted v0.2 work. The [source inventory](../integration-sources.md#squint)
records exact paths and limits. A frozen committed-source testkit build exposes two immediate
[discovery/scripted-cost blockers](squint-discovery-blockers.md) in that historical revision.
Both are fixed by `a0a2dde` and independently verified by the host's scripted coordinator and
two-tool browser checks. [Estimate journal preservation](squint-estimate-journal.md) remains open.
Five use-case tools now match description/call/result names. Serve, resources, progress/cancellation,
cap/session enforcement and estimates are implemented. `estimate` calls `finished()` and creates
zero-spend run records visible in the tool's runs resource. Commit `ae6cfb8` moves the execution
permit to checks and handles queued cancellation without allocating a run.

The testkit-only `serve --provider-script` already selects ScriptedProvider before credentials,
with serve_check_cli/serve_progress_cli tests. No request to build this again is needed. The
production provider path remains configured/authenticated; offline checks use the tool's testkit.

## S1 — Callable tasks, readiness, and supported overrides

**Request:** let a generic client discover, configure, estimate, call, and inspect each supported
check without knowing the judging loop or parsing Squint-private namespaces.

1. **Resolved:** the five use-case names align across description, tools/list, calls and results.
   Retain that conformance and its tests; no new naming adapter is requested.
2. Define bootstrap behavior without a config. `DescribeArgs` currently requires `--config`;
   Wafflery must distinguish installed-but-unconfigured from unavailable and explain how to
   configure it. Choose a supported discovery/config-selection interface with C1.
3. Declare required/optional candidate/source/reference slots, brief and rule requirements, and
   the actual supported selectable defaults/overrides. The host must not read `x-squint` or
   parse private config files to build controls. Detailed rule schemas need enough structure
   for supported inputs or a documented typed editor, not guessed fields.
4. Existing estimate/cap/session behavior is implemented. Coordinate the remaining shared
   C1/C4 request/result mapping and session identity semantics using the [C4 v0.2 proposal](C4-v0.2-proposal.md).
   Wafflery keeps estimate runs out of user history by tracking its own control requests. Missing credentials, invalid authentication, and insufficient budget
   are separate outcomes; key presence is not a provider health check or account-balance API.
5. Keep findings, evidence, locations, and verdict in the tool report plus shared result fields.
   A completed `fail`/`unsure` judgment is not a failed execution. Do not advertise internal
   evidence tools as standalone operations unless they genuinely expose that interface.

**Accept:** a generic independent client discovers a supported source-fidelity use case, provides
its declared slots/requirements, obtains the correct estimate, runs against a scripted provider,
and inspects findings and the tool report. Verify ready/missing-config/missing-key/provider-failure
states, unsupported overrides, cap refusal before spending, cancellation with spend, monotonic
progress, and reconnect through resources without another call. AC03/04/07/08/09/10.

**Deliver:** coherent callable descriptions/schema fixtures, readiness/configuration mechanism,
estimate fixtures, offline independent-client evidence, and remaining exceptions after M7.

## Pipeline handoff and report embedding

- C2: the host records outgoing MCP calls, but needs effective settings, provider/model/rule
  versions and supported CLI mapping to reproduce a selected check. A report-export command
  does not by itself export the invocation or its required image inputs.
- C3: `report view` currently prints a human sentence containing the URL. Agree machine-readable
  readiness and viewer lifetime/framing with Rascal and the host. Keep richer evidence/report
  rendering inside Squint. Clarify how reports/assets survive relocation or host-managed history.
- Required candidates/references should be portable through the example export, with original
  identity and setup names preserved and credentials excluded. Verify replay with scripted
  providers; do not promise deterministic live judgments.

## Launcher coordination

The `fcde474` source includes the H1 launcher: `WAFFLERY_BIN`, PATH, then `~/.local/bin`,
invoking exactly `--tool squint`. Retain the [agreed launcher semantics](H1-launcher-proposal.md):
exit behavior and host-owned attachment. Wafflery exercised the actual `a0a2dde` launcher
using `scripts/dev-pipeline.ts`; it opened a separate two-tool offline workspace and its host
answered the control status check. Browser journeys separately verify both tools' run/history flow.
Production installer publication and MCP configuration/registration remain separate subgates.
The first browser screen is Squint's workspace/setup state after Wafflery is installed and
running. No UI is embedded in Squint and no host-install screen appears inside Wafflery.
Use fake executable/installer tests for yes, no, failure, and already-running-host cases. AC01/02.

## Verification and response

Use Squint's scripted providers, independent MCP client and server tests already present, contract validation,
and existing repository checks. No live paid verification is required for these handoffs.
Return the implemented revision, fixtures and callable examples, test evidence, and any
unsupported choices to the Wafflery spec before integration is treated as complete.

## Sources checked

- [CLI description](../integration-sources.md#squint).
- [Shared description/result construction](../integration-sources.md#squint):
  `description`, `params_schema`, and result action attribution.
- [M7 plan](../integration-sources.md#squint): 7.5–7.10.
- [Report command](../integration-sources.md#squint).
- [Shared requests C1–C4/H1](table-manners.md) and [product acceptance](../../ACCEPTANCE.md).
