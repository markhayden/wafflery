# M4: Squint entry and a shared two-tool workspace

Authorized continuation, 2026-10-31. Preserve H1, the exact table-manners 0.1.0 gate and the
fresh `may_spend: false` dispatch check. Both sibling repos are actively implementing the 0.2
draft; verification uses isolated committed-source builds, never their mutable build products.

## Checkpoint: discovery blocked, 2026-10-31

**Superseded by the follow-up verification:** Squint #2 is closed; isolated `a0a2dde` passes
discovery and a scripted check with zero spend and complete evidence. The two-tool browser
journey passes in Chromium/WebKit. Estimates execute but lack a readable journal, so their
complete-evidence gate remains open. No contract or spend guard was changed.
See [the verification checkpoint](../reviews/2026-10-31-squint-offline-checkpoint.md) for exact
test coverage, the actual H1 launch, and retained release gates.

The isolated `fcde474` testkit build passes offline CLI estimation but fails MCP discovery:
`#/$defs/Kind` points to definitions embedded beneath `requirements.items`, not the schema root.
G05's optional coordinator test reproduces this failure; its execution/evidence assertions have
not been reached. G06 and G07's result UI work remain pending until discovery is valid. Do not
build an adapter around the invalid schema or mark this slice integrated.

Completed host work: contextual schema errors and a required two-registration regression proving
an invalid declaration cannot prevent another tool from running and preserving evidence.
Owned Squint config/script and the optional end-to-end regression are ready for the corrected
binary. See [the focused handoff](../handoffs/squint-discovery-blockers.md).

## G05 — Real Squint discovery and free estimation

Use an explicitly selected Squint testkit binary, owned fake-provider configuration/script,
cleared environment and isolated output directories. The catalog model permits a real offline
estimate; scripted provider configuration is not evidence that the action is declared free.
At Squint `fcde474`, all five checks still declare `may_spend: true` and must be refused.

Updated acceptance after #2: validate truthful free declarations and run a scripted source check
with zero spend and complete evidence; retain the existing generic paid-dispatch regression.
The original paid declarations must no longer be assumed for the fixed scripted provider.
For estimation,
execute `estimate` with candidate/source fingerprints, preserve its zero-spend result and inputs,
and verify originals are unchanged. This is an optional integration test, skipped unless a binary
is explicitly selected. Do not represent a catalog estimate as a live provider bill.

## G06 — Browser and two-tool development entry

Reuse explicit registration and the generic form. Extend test setup for Squint and for Rascal
plus Squint in one workspace. A helper accepts explicit paths, builds only Wafflery, prepares
owned tool directories and enters via the actual Squint H1 launcher. It prints the existing
library's reopen command and the limitations. No installation, keys or live providers.

Acceptance: switch tools without mixing drafts/results; discover Squint actions; run the free
scripted source check; inspect/export it, save/open an example,
and recover it from history after restart. Required reference arrays retain their later contract
gate; the tested source-fidelity estimate uses candidate and source only.

## G07 — Data-only results and remaining handoffs

If the real estimate exposes image-only assumptions, correct the generic result view based on
actual output presence and terminal status, never Squint-private fields. Keep full data inspectable.
Record evidence and prepare a focused handoff for truthful testkit cost declarations; no sibling
code edits or silent contract changes. Full image-check integration is not complete until runnable
metadata and any required input shape are supported.

Verification: optional real-tool coordinator/browser probes, relevant unit and regression suites,
embedded-asset verification when UI changes, and independent code review before merge. Native
Safari remains owner verification; this slice uses automated Chromium and WebKit.
