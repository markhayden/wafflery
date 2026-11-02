# V19 UI fidelity review

Status: audit complete; implementation slices V19b–V19d open. The coordinator authorized
starting this pass while V18c waits for the approved Squint report-style fix. This review
neither closes V18c nor weakens its console check.

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
