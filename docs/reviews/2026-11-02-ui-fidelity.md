# V19 UI fidelity review

Status: V19a/b complete; V19c/d remain. The coordinator authorized
starting this pass while V18c awaited the Squint report-style fix. V18c subsequently passed
against approved Squint `54b1f41`; this visual pass does not weaken its console check.

## Evidence and gaps

Captured at 1440 × 1000 in Chromium from local main `99e4221` and the checked-in
`work/mockups.html` (Ready workspace, dark appearance). The combined mock includes the
activity hierarchy, journey, examples, drawer, comparison and wipe studies. Current screens
use the independent sample peer or synthetic Storybook data. Grey blocks in the inspector
capture hide transient IDs and raw evidence; no local paths or session capabilities are published.

| Area | Current evidence | Mock counterpart | Decision |
| --- | --- | --- | --- |
| Hierarchy and spacing | [Workspace before](v19-images/before-workspace.png) | [Focused picker](v19-images/mock-workspace.png) | Move Tool into the application header; use the tool name/purpose as the workspace heading. Give Action its own subordinate row. Quiet the library controls and allocate more width to Result. V19b. |
| Typography and navigation | [Inspector before](v19-images/before-inspector.png) | [Mock drawer](v19-images/mock-examples.png) | Evidence buttons wrap Reports onto a second desktop row. Use compact underline navigation with accessible focus and reachable horizontal overflow on small screens. V19c. |
| Report states | [Report before](v19-images/before-report.png) | [Mock drawer](v19-images/mock-examples.png) | Keep the shared drawer, compact its controls, and retain explicit open/close/recovery and diagnostics. Screenshot empty/starting/ready/error states. V19c. |
| Examples | [Examples story before](v19-images/before-examples.png) | [Mock examples](v19-images/mock-examples.png) | The story exposes styling coupled to Workspace's stylesheet. Give examples owned styling and distinct saved-entry rows; preserve opening as a new draft. V19d. |
| Colour and iconography | [Workspace before](v19-images/before-workspace.png) | [Focused picker](v19-images/mock-workspace.png) | Background/surface/text tokens already match. Keep the approved waffle mark and quiet power icon. Use the same chevron treatment in compact and full pickers. Avoid a decorative palette rewrite. V19b. |
| Empty/loading/error and preview | [Workspace before](v19-images/before-workspace.png), [report before](v19-images/before-report.png) | [Focused picker](v19-images/mock-workspace.png) | Preserve explicit state copy; make empty results read as a deliberate preview surface. Verify wipe, mismatched frames and missing assets without inventing data. V19b–d. |
| Responsiveness | Existing `workspace-layout.spec.ts` covers 1440/1024 desktop, 320 narrow and 1280×500 short windows | Mock's single-column narrow layout | Preserve fixed desktop footers and independent scrolling. Keep page scrolling on narrow/short windows. Add screenshots and header/navigation overflow checks. V19b–d. |

## Deliberate differences from the studies

- The host renders declared **Actions**. Operations, task recipes and their grouping wait for a
  shared profile; a mock does not authorize invented tool capabilities.
- Library History and Examples remain drawers. A navigation item that opens a dialog is a
  button, not a tab pretending to switch an in-page panel.
- Paid runs keep the real cap, bound estimate, readiness and session controls. Mock budget
  values and optimistic readiness are illustrative only.
- Reports remain tool-owned, separately framed content. The host must not restyle their HTML
  or weaken CSP to match a mock. Explicit recovery/force-stop controls remain where required.
- Preserve 44px control targets, keyboard focus, modal focus return and outside-click close.
  Compact visual styling must not shrink the hit target.
- Keep the approved monochrome primary action; the mock's purple is not a new brand decision.
- No bundled examples are claimed until actually supplied. No fake CLI export replaces the
  recorded MCP call and evidence archive.
- Existing native Safari/manual-provider limits remain separate from browser automation.

## Verification and rollback

Audit capture passed against the running host and built Storybook. The V18c default matrix
also passed before this documentation-only branch (198 browser cases, eight opt-in skips).
Each subsequent visual slice runs its own full matrix, screenshot review and code review.
Screenshot regressions use synthetic fixture states; real report screenshots and traces stay
local. Revert visual slices newest first without reverting the V18 protocol work.

## V19b: workspace hierarchy

The shared frame now places Tool beside the brand; the selected tool name/purpose heads the
workspace and Action has its own subordinate picker. Library/setup actions stay reachable as
quiet buttons. Results get three fifths of the panel width and a deliberate empty preview
surface. The same picker owns search, selection and focus return in compact/full presentations.
Button targets have an explicit 44px floor (2.75rem alone was 41.25px at the existing root size).

The new layout regression failed first on both desktop and mobile because Tool was outside the
header. Targeted Chromium/WebKit checks then passed for header hierarchy, keyboard restoration,
long input scrolling, result/run visibility, short/narrow page flow and disconnected states.
The [desktop](../../ui/tests/browser/workspace-fidelity.spec.ts-snapshots/workspace-1440-chromium-darwin.png)
and [mobile](../../ui/tests/browser/workspace-fidelity.spec.ts-snapshots/workspace-390-webkit-darwin.png)
baselines use the live independent peer with no runs, paths or capabilities displayed. Both
browser baselines are reviewed; differences above 0.1% of pixels fail rather than auto-update.
WorkspaceFrame and compact Picker stories cover long tool names and connection states.

The shell extraction and its two consumers land together to avoid two competing headers.
No setup, estimate, dispatch or saved-evidence logic changed. The full matrix and review gate
remain required before this slice is marked complete.

Full shared-lock matrix passed for V19b: 243 Rust tests (four opt-in ignored), 53 UI unit tests,
two asset tests, 202 Chromium/WebKit cases (eight opt-in skips), fmt/clippy/typecheck, Storybook,
byte-identical UI rebuilds, check-ui and release build. Independent code and four-baseline visual
review approved. Rollback reverts the shell/consumer/style slice together; protocol work remains.
