# Tool setup and live session spending checkpoint

Implemented 2026-11-01 on `feat/tool-session-setup`, based on `83d0bd3`.

## Result

The shared Tool setup drawer displays declared credential-variable names and a validated live
v0.2 session ledger. It distinguishes uncapped allowance, zero balance, reservations, reported spend
and unresolved charges. Refresh preserves the draft and session identity. A failed refresh clears
the allowance; a vanished tool displays an unavailable state instead of unrelated saved evidence.

The authenticated resource-only endpoint reads the owned tool while an action is running. Setup
never dispatches, stores keys, restarts a tool, changes budget or creates history. Paid dispatch
remains refused pending C1. No dependencies, database schema or shared contracts changed.

## Verification

- New endpoint test failed with 404 before implementation, then passed. Targeted Rust checks:
  17 execution regressions, one browser authentication boundary and one session API integration
  test passed (19 total). The integration test reads during a slow run without extra dispatch.
- UI typecheck and 12 unit tests passed, including ledger validation and small-charge display.
- Chromium/WebKit related suite: 20 passed initially; two lost-tool cases failed during test
  teardown after their UI assertions passed. Draining intercepted state reads before host shutdown
  fixed teardown. All eight setup cases passed on the final rerun. Together these cover 22 distinct
  related browser cases; this is not a claim of a single uninterrupted 22-green run.
- The lost-tool regression first reproduced Inspector fallthrough with an existing saved run.
  The explicit setup branch fixes it. Other cases cover live refresh, unavailable/recovered ledger,
  retained draft, Escape/focus restoration, responsive rendering and disabled paid execution.
- Visually inspected the live drawer screenshot; requirements and two-column spending metrics fit
  the existing drawer. Storybook contains capped, uncapped, exhausted, loading and error states.
- Formatting, diff, Clippy all targets/features with warnings denied and embedded asset checks
  passed. Two clean UI builds match byte-for-byte. The release binary built successfully.
- No new native Safari claim. The owner's preceding "looks good" accepted the prior v0.2 preview;
  the browser was not specified.

## Review and external dependency

Independent reviewer (independent) identified the lost-tool fallthrough, then approved the fix
and regression with no remaining findings. Review was read-only; tests were run by the implementer.

[C1-paid-setup.md](../handoffs/C1-paid-setup.md) specifies readiness/configuration, estimate binding,
per-call caps and fixtures required from the tool/contract owners. It is a local handoff, not an
amendment or public post. User authorization for paid testing already exists; an API key alone does
not supply the missing interfaces.
