# V19 UI fidelity review

Status: V19a–d complete locally; final V19d hosted CI is left to the coordinator. The coordinator authorized
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
The [desktop](../../ui/tests/browser/workspace-fidelity.spec.ts-snapshots/workspace-input-1440-chromium-darwin.png)
and [mobile](../../ui/tests/browser/workspace-fidelity.spec.ts-snapshots/workspace-header-390-webkit-darwin.png)
baselines use the live independent peer with no runs, paths or capabilities displayed. Both
browser baselines are reviewed. The cross-machine capture policy below supersedes the initial
full-page 0.1% comparison.
WorkspaceFrame and compact Picker stories cover long tool names and connection states.

The shell extraction and its two consumers land together to avoid two competing headers.
No setup, estimate, dispatch or saved-evidence logic changed. The full matrix and review gate
remain required before this slice is marked complete.

Full shared-lock matrix passed for V19b: 243 Rust tests (four opt-in ignored), 53 UI unit tests,
two asset tests, 202 Chromium/WebKit cases (eight opt-in skips), fmt/clippy/typecheck, Storybook,
byte-identical UI rebuilds, check-ui and release build. Independent code and four-baseline visual
review approved. Rollback reverts the shell/consumer/style slice together; protocol work remains.

## V19c: evidence drawers

Inspector and pipeline views now own their shared stylesheet. Their compact underline navigation
stays on one row, with horizontal scrolling and focus kept inside the viewport on narrow screens.
These remain ordinary pressed buttons: they do not claim ARIA tab behavior without implementing
its arrow-key model. The drawer's close target is explicitly 44px wide.

New standalone evidence stories use synthetic saved records and never contact a host. The layout
regression caught wrapped evidence navigation at 390px before styling changed. Sixteen targeted
Chromium/WebKit tests pass, including outside-click/focus behavior, report open/close, recovery
confirmation and the new inspector/pipeline screenshots. Report empty, starting, ready, failed
and unavailable stories now have screenshot assertions at 320/1440px, alongside geometry checks
at 768px. Host report ownership, iframe policies, explicit actions and error copy are unchanged.

[Inspector after](../../ui/tests/browser/evidence-fidelity.spec.ts-snapshots/inspector-1440-chromium-darwin.png)
· [Pipeline narrow](../../ui/tests/browser/evidence-fidelity.spec.ts-snapshots/pipeline-390-webkit-darwin.png)
· [Report error narrow](../../ui/tests/browser/report-panel.spec.ts-snapshots/report-failed-320-webkit-darwin.png).
Full shared-lock matrix passed: 243 Rust tests (four opt-in ignored), 53 UI unit tests, two
asset tests, 206 browser cases (eight opt-in skips), fmt/clippy/typecheck/Storybook/check-ui and
release build. Independent code and representative screenshot review approved. One low-priority
review suggestion is resolved in V19d: let the close control grow with larger font settings
while preserving its 44px minimum. The current normal-size layout already meets that minimum.

## Hosted-runner visual baseline correction

Hosted run 41432529164 differed by 4,371 pixels. The trace and failure image showed a classic
scrollbar reserving about 15px in the input pane, where the local overlay scrollbar reserved
nothing. That changed line wrapping and displaced the form; raising the tolerance alone would
have accepted a real geometry difference between capture environments.

All V19 captures now share `visualPolicy`: per-pixel threshold 0.2 and at most 1% differing
pixels. Workspace chrome, context, action, input and result are separate regions; evidence is
cropped to the drawer and reports to their panel. Large unrelated page backgrounds no longer
dilute the comparison. No content is masked: these fixtures have no variable IDs/times/images.
A capture-only stylesheet removes native scrollbars and gutters, whose appearance is owned by
the OS. Product CSS and native scrolling during interaction tests are unchanged. This follows
[the specified overlay/classic gutter distinction](https://www.w3.org/TR/css-overflow/#scrollbar-gutter-property).
Browser baselines remain separate; they are never automatically accepted in CI. Regeneration
with `--update-snapshots` must be followed by a plain run: positive comparisons can update
files during regeneration, while the negated mutation comparisons never do.

The desktop Chromium test forces a 24px classic gutter, asserts that it actually occupies
space, then compares every region to the same baseline under capture normalization. WebKit
retains overlay behavior despite that scrollbar rule, so this forced-classic probe is Chromium
only. Both browsers also prove that a 24px internal action-label shift and a surface colour-token
mutation differ from the approved action baseline at the normal tolerance; after restoration,
the original must match. Desktop alignment/40:60 widths, mobile stacking, keyboard focus and
native overflow checks remain independent of screenshot tolerance. Very subtle changes below
the threshold are intentionally outside this pixel oracle; it does not promise to detect every
possible one-pixel or near-identical colour change.

CI uploads expected, actual and difference PNGs alongside the existing traces on failure.
This correction applies to V19b and V19c together; V19d must reuse the same policy.
Verification: 60 repeated visual cases passed without snapshot updates. Full shared-lock
matrix passed: 243 Rust tests (four opt-in ignored), 53 UI unit tests, two asset tests,
206 Chromium/WebKit cases (eight opt-in skips), fmt/clippy/typecheck, Storybook, reproducible
UI assets, check-ui and release build. The typecheck first caught a test-only DOM annotation;
it was corrected before completing the remaining matrix. Independent review found no required
code changes. Hosted macOS run 41436171703 subsequently passed on `6f49d7b`, including Chromium and WebKit. Rollback this test-only correction as a single commit.

## V19d: examples and preview states

Examples and cleanup now share an owned library stylesheet instead of depending on Workspace's
CSS import. Saved examples appear first as full-width named rows with tool/action context; the
selected-run save form is separated below them. Opening still fills a draft and never dispatches.
Loading, empty and failed listings have distinct states, so a pending or failed request no longer
claims the library is empty. Listing, opening and saving errors are separate: opening failures
stay with their row, saving failures stay next to the form button, and failed saves do not
erase a listing error or hide a legitimately empty library. A successful save followed by a
failed list refresh reports the save as successful and clears the form, avoiding a misleading
retry that could create a duplicate. The loading-state and action-error
placement and save/refresh-status regressions failed before their corresponding changes.

Standalone drawer stories cover saved/empty/loading/unavailable states. Comparison stories and
region baselines cover wipe, mismatched frames, missing images, read failures and loading at
320/1440px in Chromium and WebKit. The existing preview controls already match the approved
wipe/comparison studies, so no image-coordinate or comparison behavior changed. The close target
now grows with text size while retaining its 44px floor; a 24px root-font test verifies a 66px
square target. Pipeline navigation also has an explicit narrow-screen keyboard reachability check.

[Saved examples](../../ui/tests/browser/library-fidelity.spec.ts-snapshots/examples-saved-1440-chromium-darwin.png)
· [Narrow examples](../../ui/tests/browser/library-fidelity.spec.ts-snapshots/examples-saved-320-webkit-darwin.png)
· [Wipe](../../ui/tests/browser/library-fidelity.spec.ts-snapshots/compare-wipe-320-chromium-darwin.png).
All new baselines use the shared region/tolerance/scrollbar policy above. Focus outlines remain
visible when the modal initially focuses its heading. Example summary rows contain metadata;
they do not invent thumbnail references absent from that summary. The mock's fixture catalogue
is still not advertised as an installed examples library.

Verification: the loading, row/form error and save/refresh-status checks each failed before
their fixes. Targeted browser checks then passed with reviewed baselines, followed by a plain
full shared-lock matrix: 243 Rust tests (four opt-in ignored), 53 UI unit tests, two asset tests,
220 Chromium/WebKit cases (eight opt-in skips), fmt/clippy/typecheck, Storybook, byte-identical
UI builds, check-ui and release build. Independent review approved after the error-path fixes.
The preceding visual-runner correction passed hosted run 41436171703. This final slice's hosted
run remains the coordinator's next check. Rollback reverts owned styles, consumers, stories and
tests together; saved library records and the v0.3 protocol are unchanged.

The approved V19 gap list is addressed within the deliberate differences above. Native Safari's
report/frame/restart walkthrough and an actual billed-provider check remain explicitly separate
limits in the V18 checkpoint, not claims made by this visual pass.
