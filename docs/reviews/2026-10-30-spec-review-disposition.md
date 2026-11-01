# Specification review: disposition

Historical snapshot. Current source facts and plan corrections are recorded in the
[2026-10-31 disposition](2026-10-31-plan-review-disposition.md).

Date: 2026-10-30. [Original review](2026-10-30-external-spec-review.md).
The owner supplied an external review of ten numbered document snapshots, including
table-manners CONTRACT.md. No application tests, schema inspection, or live tool verification
were performed during that review.

The author checked findings against the current specification and table-manners v0.1.0
`d5a2d8e`. Changes below are specification corrections, not evidence of working software.

| Finding | Disposition and correction |
|---|---|
| 1. Missing attempt/run association | Accepted. Persist request/progress-token mapping, bind explicit run references, add host-only rejected state, and retain unknown outcome before association. Never infer a run from recent-list timestamps. C4 now requests the exact progress-reference wire format and early-cancel semantics. |
| 2. Undefined preservation completeness | Accepted with qualification. Define the v0.1 evidence set, but do not redefine full preservation as that subset. Native/report categories remain explicit. A mask fingerprint is not a locator; missing referenced bytes remain incomplete evidence. |
| 3. Relocation blocked by native workspace paths | Accepted as a design risk; the claim that every real Rascal run necessarily blocks relocation was not proven. Move proposed stable tool storage outside the relocatable owned library. Disclose separate usage and that tool bytes stay put. Opaque references into relocated paths still require validation. |
| 4. Generated client program required | Clarified. Prior wording was conditional, not proof a code generator was required. AC17 now explicitly requires captured MCP call/setup data; optional generated programs need verified dependencies/replay. Supported CLI/bundle replay remains a first-release requirement. |
| 5. Estimates on a busy serial tool | Accepted. No host history rows for control estimates; no queued estimates while busy; sequential estimate/execute under one host reservation. Separate discovery refresh from explicit configuration/session restart. C1/C4 asks tool-side resource/serialization semantics. |
| 6. Launcher ownership ambiguous | Accepted clarification. Tool launchers only locate/install/invoke Wafflery. The Wafflery binary alone owns locks, auth, attachment, and browser opening. |
| 7. Same-origin asset execution | Accepted. Verified raster allowlist, explicit types/nosniff, attachments for unsupported active content, escaped structured views, restrictive UI CSP, separate report origin, and malicious-file fixtures. Detailed HTTP security design remains open. |
| 8. Unplanned prior-history import | Accepted scope clarification. First-release history tracks host-started attempts. Importing unrelated CLI/agent history is deferred; run resources still support known attempts. |

## Simplification recommendations

- Use a per-run viewer idle timeout refreshed by visible inspection activity; remove durable
  per-tab lease bookkeeping. Quit still owns shutdown. Actual report protocol remains C3.
- Specify disposal of execution copies after confirmed termination and preservation. Require
  ownership to be gone before sweeping crash leftovers. Ordinary copying is enough; APFS cloning
  is optional, avoiding another platform-specific requirement before measurement.
- Retain detectable-unstable-input checks: accepted AC11 says changed originals must not
  silently substitute bytes. A digest of a concurrently torn copy alone cannot prove stable input.
- Do not add a periodic asset verification service. Verify publication and relevant use/export
  boundaries; missing or corrupted assets must still be identified honestly.
- Stage fake support against existing contract fields. Use a neutral Actions catalogue for
  unclassified v0.1 calls in the development milestone and labeled Storybook fixtures for future
  task/operation presentation. Do not categorize utilities as Operations merely to fill that tab.

## Readiness after correction

The corrections remove ambiguities in the host-side draft. They do not resolve C1–C4/H1 or
approve a full implementation plan. A narrow v0.1 fake-tool milestone can be planned once G6's
runtime/build/test choices are concrete. C4 must clarify progress run-reference encoding before
that particular integration path is called conforming; result-only and unknown-outcome paths
can be tested without inventing fields. Full release requirements remain AC01–AC22.

No additional owner product decision is required by this review. Neutral interim presentation
is a development sequencing choice, not a replacement for the accepted focused picker. The
owners still need concrete wire agreements, which are already collected in the handoff.

## Verification

Checked revised Markdown links/fences, decision-log continuity, and contract references. No
application implementation or sibling repository was changed. No runtime conformance or browser
tests apply to these documentation edits, and no paid tool execution was performed.
