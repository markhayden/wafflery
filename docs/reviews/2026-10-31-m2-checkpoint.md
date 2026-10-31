# M2 checkpoint: offline actions and immutable evidence

Status: complete through `efa7427`. F09–F13r are implemented. M3 remains authorized and pending.
No paid calls, sibling edits, installs, pushes or publishing were performed.

## Working behavior

Explicit local descriptors start owned MCP children with cleared environments. The client pins
MCP 2025-11-25 and table-manners 0.1.0, validates live declarations and refuses paid, malformed
or stale action metadata. The independent sample-lab peer does not use the host SDK.

The generic form, shared picker, comparison viewer and inspection drawer execute a real offline
image action. Inputs, actual SDK-assigned wire identity, raw response, verified outputs and journal
are saved independently of the editable draft. Cancellation without a structured run identity stays
unconfirmed while the process may work. Events reconnect and update other tabs; closing a tab
does not cancel a run. Restart preserves evidence and never retries interrupted work.

## Verification

- Rust formatting and all-target/all-feature clippy pass with warnings denied.
- All-target Rust tests: 48 passed; the optional real Rascal smoke is ignored by default.
- UI typecheck and 10 unit tests pass. Storybook builds.
- Chromium/WebKit: 38 browser tests pass, including raw-response inspection, intermediate
  progress in a second tab, reconnect, cancellation, host restart and keyboard comparison.
- Two clean UI builds are byte-identical; the release binary builds from embedded assets.
- Dependency audits: 218 Rust crates against 1,395 advisories and 259 UI packages; no findings.
- Opt-in Rascal smoke passed against local binary 0.1.0, repository revision `f8fcea2`.
  A no-config/no-filler seam call used an owned panorama fixture and reported zero spend.
  Output fingerprints/dimensions/journal were verified and the original remained unchanged.
  Required location bindings prevent mask_rect in this foundation, so the smoke uses seam.
  Image coordinate metadata follows the published image-reference schema, not a tool-specific rule.

## Review and corrections

Independent gpt-6-astra review found stale browser declarations and reservations retained after
child exit. Regression tests first reproduced both; refreshed declarations now publish and owned
process status is rechecked on snapshots and quit, including delayed exit after cancellation.
The same review verified progress forwarding and raw-response persistence. These were missing
from the initial implementation and now have direct browser/regression assertions. Rejected
submissions invalidate after releasing their reservation. Run waits for required image imports,
fixing a race found by WebKit. Registration reads are bounded before allocation.

Final independent review found no remaining important M2 issues, contingent on the now-passing
Rust checks. The review-fix commit touches more than five files because tests and generated UI
bytes cross the lifecycle boundary; it contains no unrelated feature work.

M3 replaces full-record recent snapshots with compact paginated history, adds examples/baselines,
evidence archives, reference-aware cleanup and explicit active/force shutdown. Current exports
and native report preservation remain unavailable. No production installation, provider credentials,
paid execution or full operations profile is claimed. Native Safari verification remains an M3 check.
