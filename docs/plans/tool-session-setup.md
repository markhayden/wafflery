# Tool setup and live session spending

Completed 2026-11-01. See the [verification checkpoint](../reviews/2026-11-01-tool-session-setup.md)
and [remaining C1 owner handoff](../handoffs/C1-paid-setup.md).

Implements the currently supported part of SPEC Q9 / AC08. The owner has authorized continuing
integration and paid testing; the table-manners contract must not be changed by this work.

## Current boundary

At table-manners `0f04647`, v0.2 remains the released contract. C1 readiness and estimate binding
remain proposals for v0.3. Rascal `2f5e3dc` and Squint `e8d1604` have additional v0.2 follow-ups;
those commits do not create shared provider setup or an input-bound estimate interface.

## Deliverable

A shared Tool setup drawer displays declared credential-variable names and a freshly read,
validated tool-session ledger. It never collects keys or treats a variable name as a connected
provider. Show capped, uncapped, reserved, spent and unresolved amounts accurately. Refresh reads
the same process; errors clear the current balance display rather than presenting stale allowance.
Opening or refreshing setup never dispatches actions, creates history, restarts a tool or changes
its budget. Existing paid-dispatch guard remains enforced. No new dependency or database schema.

## Increments

1. Authenticated GET /api/tools/{id}/session through the existing coordinator/session validator.
   Verify unauthenticated/missing tool rejection and reads while a run is active, no extra runs.
2. Typed browser reader and shared session/setup view with capped/uncapped/loading/error stories.
   Verify honest credential status, zero versus unknown, six-decimal currency, refresh failures.
3. Integrate with existing drawer and selected tool/action. Browser-check real endpoint, keyboard
   closing, no dispatch, responsive layout, and paid action remains disabled.
4. Record exact C1 owner acceptance requests in a local handoff. Review, build and commit.

## Checks

`cargo test --locked --test tool_session_api`; `bun run --cwd ui typecheck`;
`bun run --cwd ui test:unit`; targeted Chromium/WebKit and Storybook tests, then related Rust
regressions. Rebuild/check embedded assets, Clippy, release build and independent review before merge.

Paid-provider configuration and estimates/caps remain an external interface dependency. Keep
working on supported host/UI pieces; no tool-private-field adapters and no unapproved public posts.
