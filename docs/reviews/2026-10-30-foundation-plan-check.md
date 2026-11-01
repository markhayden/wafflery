# Foundation plan consistency check

Historical snapshot. Current source facts and plan corrections are recorded in the
[2026-10-31 disposition](2026-10-31-plan-review-disposition.md).

Date: 2026-10-30. Author review of [PLAN.md](../../PLAN.md) and
[foundation runtime](../design/foundation-runtime.md), not a second external review.

## Findings addressed while planning

- **Global gates blocked unrelated work.** Q19 and the acceptance gate section now scope
  C1–C4/H1 to integrations; M0–M3 can be reviewed independently.
- **SDK default protocol differs from the contract.** Inspected rmcp 3.8.0 model source defaults
  to 2026-07-28. Foundation explicitly negotiates only 2025-11-25 and tests with an independent peer.
- **SDK convenience shutdown could violate explicit force quit.** Inspected child-process source
  has a three-second forced shutdown. Own the child handle and use the SDK async read/write
  transport; preserve host cancellation/shutdown policy.
- **A different loopback port does not isolate cookies.** Use explicit bearer API authorization
  and authenticated Blob fetches; no ambient cookie/session authority is sent to a report server.
- **A staged fake could be mistaken for full conformance.** Document the subset and require C4
  progress-reference semantics before claiming the complete progress/cancellation path.
- **Source builds could silently serve stale UI.** Commit prebuilt assets intentionally, include
  deterministic source/output digests, verify without invoking a JS toolchain from cargo.
- **Platform claims exceeded evidence.** Foundation validation targets the available ARM macOS
  machine. Public OS/architecture support belongs to the actual packaging tests, not inference.

## Plan assessment

Ready for owner review of the scoped foundation. Tasks have dependencies, acceptance outcomes,
named verification and commit boundaries. Large transport/UI tasks explicitly split into coherent
commits. Checkpoints deliver an import loop, execution loop, then reusable experiments. No
paid calls, installer releases or sibling edits are included. Full release acceptance remains open.

## Verification performed

Checked local links and fenced blocks in the seven new/updated planning documents; Q1–Q19,
F01–F18, four milestone checkpoints, and G1–G6 coverage. Verified proposed Bun cwd/lock flags
against installed CLI help. Checked selected primary runtime references and local Rust source.

No dependency installation, compilation, browser automation or runtime integration was performed.
Registry DNS from the shell failed; available releases were checked via official web sources.
M0 must verify dependency resolution, runtime compatibility and the actual test environment.
