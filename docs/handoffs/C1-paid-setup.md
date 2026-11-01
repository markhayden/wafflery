# C1: unblock generic provider setup and paid execution

Local owner handoff, 2026-11-01. This requests shared interfaces; it does not amend table-manners,
approve v0.3 proposals, or authorize a Wafflery-only adapter. Nothing has been posted publicly.

## Implemented on the host

Wafflery accepts published v0.2, records session/correlation identity, checks remaining-budget
arithmetic, preserves unresolved spend, and recovers associated cancelled runs without replay.
Its setup drawer reads the live session resource and displays action cost.needs_env names.
These names do not prove that credentials are present, valid, or sufficient for an action.
The user has authorized paid testing; lack of permission is not the blocker.

## Missing shared behavior

1. **Unconfigured discovery and readiness.** An installed tool must describe itself without
   credentials/configuration. For each action, identify missing provider/model/configuration
   prerequisites and distinguish credential present, validated and unknown. Supply safe setup
   instructions or a declared setup surface without returning credential values. State whether
   rechecking readiness can make network calls or spend; a local inspection must not do so.
2. **Configuration lifetime.** Specify how a host selects an existing tool-owned configuration
   and which changes require a new serving session. No host interpretation of private config
   fields, inherited arbitrary environment, or silent restart to refresh an allowance. Secrets
   stay with the tool/OS; do not add key storage to Wafflery.
3. **Estimate dispatch and binding.** Declare the estimate operation for each paid action and
   the exact mapping from its arguments, images and configuration. Return a machine-checkable
   binding to those inputs plus tool version, effective model/configuration and session ID.
   Include worst-case cost, expiry/validity, and the effective configured cap. A model, input,
   configuration or session change must invalidate it. Estimates never spend.
4. **Per-call cap.** Define the shared input carrying the chosen cap and how the tool checks
   it together with the estimate binding and its session reservation before any paid call.
   Return structured refusals for stale estimates, exhausted allowance and an unsupported cap.
   The tool remains authoritative across concurrent clients. Null allowance is not zero.
5. **Control results.** Rascal estimates need not allocate a run; Squint estimates do. Declare
   both shapes so the host can inspect an estimate without adding it as an ordinary attempt.
   Do not require parsing action names or tool-private result bodies to identify a control.

## Acceptance fixtures requested from the contract/tool owners

- No configuration, missing credential, present-but-unverified credential, invalid credential,
  ready action, and a provider failure that remains distinct from readiness.
- Estimate for action A followed by changed image bytes, parameters, model, configuration or
  server session; every stale estimate is refused before provider dispatch.
- Cap below worst case, configured cap below requested cap, exhausted session, concurrent
  reservations and cancellation with unresolved spend. Existing v0.2 accounting still applies.
- Both estimate-result forms with their exact caller mapping; fixture proving zero provider
  dispatch during readiness/estimate inspection.
- Configuration recheck and explicit restart showing the new session boundary, with old
  unresolved spend retained in its original run history.

Please return a tagged shared schema/fixtures or a reviewed proposal with ownership and delivery
order. Wafflery can implement against the agreed shapes using its independent peer while
Rascal and Squint adopt them. An API key alone cannot fill these missing interfaces.

## Source boundary

Inspected table-manners `0f04647` (released contract remains v0.2; C1 is proposed for v0.3),
Rascal `2f5e3dc` and Squint `e8d1604`. No sibling files changed. The host still refuses
may_spend:true dispatch until the estimate/cap/readiness path is implemented and verified.
