# Squint entry checkpoint — 2026-10-31

Status: host diagnostics complete; real Squint integration blocked at schema discovery.

- Required Rust suite: `cargo test --locked --all-targets` — 63 passed, 0 failed, 2 ignored
  explicit-binary integration tests. Formatting and `git diff --check` pass.
- New two-registration regression reproduced the missing diagnostic context before the fix.
  Afterward it verifies the failing tool/schema name, no dispatch to the malformed tool, and
  a completed, preserved run through the other registration.
- Optional real Squint probe against frozen committed `fcde474`: expected observed failure,
  `MCP tool 'source_fidelity' input schema: Invalid schema: Pointer '/$defs/Kind' does not exist`.
  Estimate execution and evidence assertions remain unreached. The separate CLI estimate works;
  its envelope is not actual spend. No provider was called.
- Independent reviewer (independent): approved, no blocking findings, contingent on the
  required suite (now passed). Reviewer did not independently rerun tests.
- No UI, H1 lifecycle, dependency pin, paid-call guard or sibling working files changed. No new
  browser/Safari claim: the prior Rascal UI and owner's saved library remain in place.

Owned config/script and an ignored end-to-end probe are committed to make the next corrected
binary reviewable. They do not certify a successful check or model judgment. The two-tool
browser entry, estimate presentation and scripted-check execution remain pending in
[the slice plan](../plans/m4-squint-entry.md). Pass the
[focused handoff](../handoffs/squint-discovery-blockers.md) to Squint's owner; no message was sent
to that window by Wafflery.
