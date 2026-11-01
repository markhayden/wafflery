# M3 checkpoint: saved experiments and evidence export

Status: implementation and automated verification complete on `feat/m3-saved-experiments`;
native Safari verification is blocked. M3 has not merged to main. F14–F18u are implemented;
F18b has one remaining manual check. M4 is outside the current authorization.

## Working behavior

History uses compact summaries and cursor pagination, with full immutable evidence loaded when
a run is selected. Baselines persist per tool/action. Examples retain independent input references,
load a fresh draft without executing it, and survive deletion of their source run.

The shared pipeline drawer copies the recorded MCP request and setup information, and downloads
a bounded evidence archive with a manifest, hashes and optional verified PNG/JPEG inputs/outputs.
It accurately labels local paths and missing native replay support. It does not invent CLI flags.

Cleanup previews selected records and reclaimable image bytes, rechecks references before deletion,
clears deleted baseline pins and retains shared example assets. It never deletes original images or
opaque tool workspaces. Known materialized inputs are disposed after preservation or confirmed
process exit; ambiguous crash leftovers are retained. There is no automatic expiry.

Confirmed shutdown cancels active work, preserves terminal evidence and waits for owned tools.
Refusing shutdown leaves work running. Force termination is a separate action limited to owned
child handles. A transient status failure remains retryable and does not falsely report completion.

`bun run scripts/dev-fake.ts` builds and opens the working application with an isolated sample
library. The illustrative offline tool is clearly identified and does not claim to repair an image.
Closing a tab leaves the host running; the printed reopen command preserves that library.

## Verification

- Rust formatting and all-target/all-feature clippy pass with warnings denied.
- All-target Rust tests: 56 passed; one optional Rascal smoke remains ignored by default.
- UI typecheck, 10 UI unit tests and two manifest tests pass. Storybook builds.
- Chromium/WebKit: 54 browser tests pass, including history/restart, baseline comparison,
  independent examples, JSON draft resets, archive downloads, older-page cleanup, missing saved
  image bytes, shutdown/force and retry after a status error. The recorded outcome remains intact
  when saved image bytes are unavailable. A real result screenshot was also inspected visually.
- Two clean UI builds are byte-identical and the checked-in manifest matches the sources/output.
- Release build and Cargo source packaging pass. The extracted package builds offline with
  Node/Bun excluded from PATH; its binary serves embedded HTML/JS from an empty working directory
  and quits cleanly through its private control channel.
- Dependency audits: 220 Rust crates against 1,395 cached advisories and 259 UI packages; no findings.
- The opt-in real Rascal smoke passes against local binary 0.1.0 and contract 0.1.0. The no-config,
  no-filler seam call reports zero spend, with output fingerprints/journal verified and the original
  unchanged. This is an optional offline integration probe, not completed production registration.
- Native Safari: **not run**. Computer Use returned `Computer Use was not approved to use Safari`,
  including after the user's general permission grant. Automated WebKit is not represented as
  native Safari coverage. No alternate automation path was used to bypass that permission.

The release smoke harness initially concatenated a relative script URL incorrectly; using URL
resolution fixed the harness, and the full smoke then passed. No application change was needed.
The owned development and package-smoke hosts were stopped after verification.

## Review and corrections

Independent gpt-6-astra review found a shutdown/preservation race, retained disposable inputs,
stale JSON form state after example loading, cleanup limited to the newest history page, and
misleading shutdown status after a transient fetch failure. These have targeted regressions and
were corrected before the final suites. An atomic lifecycle transition coordinates response
preservation with transport closure; no implicit SDK kill timeout was introduced.

Browser verification also exposed a temporary image form during declaration discovery. The form
now waits for discovery, preventing uploads into a transient view. A cancellation test now waits
for actual fake-tool allocation and bounded terminal evidence rather than assuming that wire
persistence means the tool has allocated a run. Production cancellation still never scrapes text.

The final independent review reported no remaining important issue, contingent on the now-passing
regressions and final checks. Native Safari remains a separate outstanding milestone gate.

Work was split into behavior commits, review corrections, and a separate mechanical formatting
commit. Several connected API/migration/UI/test slices exceeded the approximate five-authored-file
budget; generated assets were reviewed separately. The formatting commit is intentionally larger
and adds no behavior. No remote exists, so no CI publishing or PR was performed.

## Remaining limits

This is a development foundation pinned to table-manners 0.1.0 and MCP 2025-11-25. The dispatch
guard refuses paid actions. Production discovery/installers, provider setup, operations profiles,
portable native replay, structured paid sessions and tool-owned report framing retain their later
contract/integration gates. No paid calls, sibling edits, production installs, pushes or publishing
were performed. Complete native Safari verification, then close F18b and merge this milestone.
