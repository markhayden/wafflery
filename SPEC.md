# Wafflery specification

Status: approved product decisions Q1–Q19 and implemented host behavior through M5.21,
2026-11-03. M5.22 records acceptance and remaining limits; V20 adds image-import feedback.
This is a macOS development build, not approval of installation/distribution or a public release.
The [milestone review](docs/reviews/2026-11-03-m5-operations.md) and [PLAN.md](PLAN.md) record evidence.

## Objective

Provide a refined local browser interface for exploring, running, reviewing, debugging, and
eventually evaluating visual pipeline tools. Begin with Rascal and Squint as independently
usable tools. The application must be useful with either one installed; Pixel and cross-tool
workflows come later.

The primary experience is to choose a task, provide images and a prompt where useful, run
with sensible configured defaults, inspect the result, and repeat with a change. Detailed
inspection explains what happened without making the user assemble every internal step.

## Established direction

- One local program serves the browser interface. Tool-specific `playground` commands are
  thin launchers that locate it, offer installation if necessary, and open it with context.
- The workshop reveals installed tools and relevant setup needs. Optional missing components
  should affect the activities that need them, not disable everything the tool can do.
- Tool workspaces use a restrained two-panel layout inspired by the supplied Fal screenshots:
  inputs and controls on the left, visual results on the right, deeper inspection within reach.
- Activities name outcomes. Rascal includes Repair a seam and Remove an object; masks and
  intermediate steps are available to inspect. Manual drawing is optional refinement.
- A known panorama wrap seam can be repaired automatically without a prompt. Prompt-based
  finding and free-form repair intent have different requirements and must not be conflated.
- Squint owns its judging loop, rules, evidence tools, findings, and verdict. Wafflery
  exposes the supported inputs and settings and presents its results.
- Usage documentation should be available beside the playground for CLI and MCP integration.
- The tools own their behavior and detailed report pages. Wafflery uses the shared
  contract; missing capabilities become explicit requests to the owning tool or contract.
- Prefer clear current interfaces over compatibility layers or CLI execution fallbacks.
- First usable release covers individual runs, debugging, local history, and reusable examples.
  Batch evaluations follow in the next milestone (Q1, confirmed by the owner).
- Expose all supported individual-run actions (Q2, confirmed). Resolve their grouping and
  visual hierarchy through mockups rather than fixing an Advanced category in advance.
- Focused picker is the selected layout (Q3, confirmed after the revised mockups).
  Complete tasks and independent operations have separate selectors.
- Independent operation testing is a core use case. Users must be able to inspect and test
  individual pieces deeply, not only view them as logs inside a completed task.
- UI direction: React/TypeScript with Vite, a small owned component library seeded from shadcn,
  and development-only Storybook. The Rust host embeds built assets; end users need no frontend
  toolchain. See [ADR-0001](docs/decisions/0001-ui-foundation.md).
- The playground must build confidence and provide a path into the user's code. A run-specific
  Use in your pipeline pane should expose the tested invocation, configuration, prerequisites,
  and input/artifact dependencies. General Usage documentation alone does not meet this need.
- For runs started through Wafflery, preserve owned input snapshots before execution and
  reuse identical stored files across runs. History and exports use those snapshots rather than
  depending on the original paths. This is an accepted requirement, not implemented in the mock.
- Browser tabs do not own run lifetime. Closing a tab leaves the local host and active runs
  running; reopening reconnects to their current state. Stop run cancels an individual run.
  Quit Wafflery shuts down the host and requires confirmation when work is active.

## Reviewed interface direction

Use Playground, Operations, Usage, and Runs, with examples available from the shared drawer.
Playground has a short task picker; Operations has a searchable grouped catalogue. Both use
one selector component. Detailed inspection and pipeline handoff use the same drawer across
Rascal and Squint. The owner reviewed the combined drawer/image-slider mock positively.
Shared grouping/readiness/typed-artifact declarations remain integration work, not a reason to
hardcode the mock's action lists into the application.

Depth is confirmed: the owner wants to test independent pieces. An operation workspace should
accept the operation's typed inputs, expose its parameters, run only that operation, and
inspect its actual output type (location, mask, image, profile, series, or plan). Selecting
compatible inputs from previous runs would avoid rerunning unrelated upstream work. Support
for those artifact bindings is a tool/contract requirement; it must not rely on guessed paths.

Verified locally against installed Rascal 0.1.0: `line` runs independently using an image,
`column=0.5` and `width_px=20`, and commits a Location artifact with status `ok`. It defines
geometry; it does not detect a seam, produce a mask, or paint pixels. `rasterize` can consume
that location to create a mask, and `fan` can consume its column form to measure a band profile.

The current [Focused picker mockup](work/activity-hierarchy.html) has separate Playground and
Operations views. Category strip and Activity sidebar have been retired. The operation
and task selectors share one bordered component, with matching dimensions, stacked title
and secondary text, and a chevron without a Change label.
Line demonstrates position/width controls, a Location overlay, and JSON; Feather demonstrates
a saved-mask input and independent parameters. Other outputs are typed placeholders.

The [local mockup preview](work/mockups.html) is self-contained and requires no server or
ChatGPT renderer. Interactions are simulated, not live tool outputs. DOM interaction checks
pass; browser layout verification was unavailable in the authoring session.

The preview now includes all three entry journeys, setup/readiness, progress and cancellation,
session run history, and a run-specific pipeline handoff pane. Saved invocation/configuration
information stays tied to a selected attempt when the form is edited. Unsupported invocation
exports are explicit placeholders. See [user journeys](USER-JOURNEYS.md) for the walkthrough
and the boundary between working mock interactions and future tool integration.

## Decision log

- **Q1, confirmed:** Batch evaluations belong in the next milestone. Complete individual
  runs, debugging, local history, and reusable examples in the first usable release.
- **Q2, confirmed:** Include all supported individual-run actions. The hierarchy and grouping
  remain open until reviewed in mockups.
- **Q3, confirmed:** Focused picker. The owner rejected the crowded combined menu;
  reorganize tasks and lower-level operations while retaining access to everything.
- **Q4, reviewed direction:** Separate complete tasks from independent operations.
  Remove Category strip. Subsequent review selected Focused picker and retired Activity sidebar.
- **Q5, confirmed:** Download example includes required images and supported artifacts by
  default. Show the included files and size; offer an explicit option to reference existing
  files instead. Omit credentials and tool/model installations. Unsupported artifact portability
  must be labeled. The original mock downloads only a manifest; the implemented invocation-bundle drawer carries supported input bytes.
- **Q6, confirmed:** Retain an owned copy of each run's inputs; store identical files once.
  Moving or editing originals must not change saved runs or their exports. Library scope is
  settled in Q14 and preservation scope in Q15; exact filesystem layout and the preservation
  mechanism remain to specify.
- **Q7, confirmed:** Keep history until the user explicitly deletes it. No automatic expiry
  or silent eviction in the first release. Show disk usage and provide cleanup controls.
  Cleanup preserves assets referenced by any remaining run or saved example, and never deletes
  the user's original input files.
- **Q8, confirmed:** Runs continue when the browser tab closes. Reopening reconnects without
  submitting another run. Stop run and Quit Wafflery are separate controls; quitting while
  work is active requires confirmation. Process launch/detachment and crash recovery still
  require an implementation design.
- **Q9, revised 2026-11-04 (owner):** spend is optional, never a gate. Run works for a paid
  action without an estimate or a cap; the tool enforces its own spending limits (table-manners
  4.5) and refuses before spending. A spending limit is an optional field that may lower the
  tool's cap, and Check cost an optional step; a checked cost binds the run to that price, and
  edits make the check stale (Run still works, unbound). No second confirmation. Results show reported spend
  and its measured/estimated basis, including cancelled runs. Local examples show no provider
  cost. Provider configuration/credentials are prerequisites where needed; keys alone do not
  supply pricing or account balance. The current preview uses explicitly simulated cost data.
- **Q10, confirmed:** Users can pin a saved run as a baseline and compare another saved attempt
  manually. Keep this within individual-run review; batch evaluations remain deferred. The
  mock scopes baselines to a task/operation, shows saved outputs and setting differences, and
  excludes untested form edits. It does not compute or claim a quality winner.
- **Q11, confirmed:** Examples are named reusable starting points separate from Runs. Include
  bundled starters and examples saved by the user from completed runs. Opening an example fills
  a draft form without running or reusing a previous result; Run creates a fresh attempt. The
  mock supports names and optional notes and preserves the source run's settings independently
  of later edits. Real examples must retain the assets they need even if their source run is
  deleted; persistence remains unimplemented in the session-only mock.
- **Q12, reviewed direction:** Use one shared right-side drawer for pipeline handoff,
  run details, example browsing, and saving examples across tool workspaces. Give it a consistent
  heading/context, close control, independent scrolling, keyboard focus handling, Escape close,
  and narrow-screen treatment. Keep these panels out of the primary workspace's document flow.
- **Q13, reviewed direction:** Support a draggable image comparison divider, with
  keyboard control and clear labels for the actual compared artifacts. Use Source/Result for
  compatible edited images, Source/Mask or Location overlay for those outputs, and Baseline/
  Attempt for saved-run comparison. Retain side-by-side comparison. Do not imply pixel alignment
  or an edited result when the artifacts do not support that interpretation.

- **Q14, confirmed:** One personal local library shared across projects and all three entry
  paths. Default to Wafflery's application-data folder; do not require a project to begin.
  History and saved examples are shared whether launched through Rascal, Squint, or Wafflery.
  Settings expose the location and disk usage, with an option to choose another storage location.
  Exact platform paths and safe relocation behavior remain engineering design work.
- **Q15, confirmed:** Preserve tool-declared outputs and debugging artifacts alongside inputs
  and settings, including intermediate masks, geometry, findings, and data needed for reports.
  Saved review must not depend on disposable tool files. Exclude disposable caches and model
  downloads; label unsupported preservation explicitly. Keep these assets until explicit
  deletion under Q7; automatic retention periods may be considered later. This settles scope,
  not the tool/host persistence interface or whether reports are preserved or reconstructed.
- **Q16, confirmed:** Target macOS for the first release. Linux is a possible low-priority
  follow-up; Windows is outside the initial scope. Keep portable boundaries where practical,
  without adding untested platform support. Minimum macOS version, CPU architectures, browsers,
  and release packaging remain engineering details to specify and verify.
- **Q17, confirmed:** Keep crash recovery minimal for exploratory work. Restore persisted
  history and flag attempts without a recorded terminal outcome as interrupted, with unknown
  outcome/spend where applicable. Preserve already saved evidence. Do not automatically retry,
  resume computation, or build durable job recovery in the first release. Explicit retry creates
  a new attempt. Normal browser reconnection to a live host remains required by Q8.
- **Q18, confirmed:** Use SQLite embedded in Wafflery's host for local metadata. Open the
  database on host startup and close it on shutdown; no separate database service is required.
  Data persists on disk while the host is stopped. Closing a browser tab does not stop the host
  or database. Schema, dependency selection, and detailed storage mechanics remain engineering work.
- **Q19, confirmed:** Continue foundation work independently of later tool-contract agreements.
  Scope integration gates to the features that require them instead of blocking the entire project.
  Plan the local host/library/shared UI and v0.1 fake-tool path now; keep later integrations explicit.
  This agreement authorizes foundation planning, not an unreviewed full-release implementation.

## Implementation and remaining gates

The exact live contract is table-manners `=0.5.0` (on branch `m13-checker`, by path until rascal's
PLAN.md 13.15 publishes it; `main` keeps published `=0.4.0` until the branch merges); MCP remains
`2025-11-25`. Rascal's M13 (task 13.14) renders what v0.5 adds: an action's verdict (`answers: verdict`; a judged result's `verdict` in plain
words with its places drawn), optional requirements (never a reason to need setup) and image-out
labels (the Show menu's names, saved with the run). 13.15 replaced 0.4.0 directly, with no dual support. Earlier
live contracts are refused, while saved historical evidence stays readable. The independent fake
exercises generic task/operation/utility discovery, scalar/list ports, image/location/native
bindings, indexed refusals, session grants, cancellation, invocation capture and portable bundles.
Approved real Rascal/Squint snapshots pass [M5.21](docs/integration-sources.md).

C1 readiness/estimates, C2 resolved invocation and declared native portability, and R3 typed-operation
integration are implemented within their advertised capabilities. C3 report address/framing/lifetime
and C4 explicit session/budget boundaries are verified. Nonportable artifact types remain labelled
workspace-dependent; native previews are never replacements. Tool-owned report sources still need
their original usable run directory. Broad report reconstruction and unadvertised tool capabilities
are not promised.

The workspace keeps tool choice in the header and groups declared activities below it. Port editors
bind saved artifacts and images without rerunning upstream operations. The shared drawer reviews
ordered experiment chains; each step requires its own ordinary Run (a paid step may check its cost
first; the tool's limits apply either way). This is guided replay, not unattended orchestration or a node editor.

[Acceptance gates](ACCEPTANCE.md#feature-and-release-gates) retain actual distribution work: installer
channel, automatic installed-tool discovery, supported macOS architectures/minimum version and release
verification. No Linux/Windows support or new native Safari coverage is claimed. Evaluations, Pixel,
agent loops and general cross-tool orchestration remain later scope. R2 prompt-led damage localization
remains optional and requires tool support. V20 addresses image-import latency/feedback without
changing pixels silently or guessing a tool's pixel limit.

The implementation uses Rust with embedded Vite/React assets, embedded SQLite, stdio MCP and the
owned component set with development-only Storybook. [ENGINEERING.md](ENGINEERING.md),
[README.md](README.md), and the runtime design document the actual boundaries and commands.

## Planning and documentation obligations

Review the scoped runtime design and implementation plan together before implementing that scope.
The plan must contain ordered tasks,
dependencies, validation, and a detailed commit strategy. Each checkpoint should describe a
coherent working state, its checks, and how to roll it back. Implementation follows plan approval.

At kickoff the folder contained the brief, interface map, tool requests, and an earlier interface
study. The project is now versioned in Git; README documents the review mock and specification
package. Check for repository instructions throughout the work. Keep README and design documentation aligned with
implemented behavior.

## Supporting documents

- [First-release acceptance criteria](ACCEPTANCE.md)
- [Original brief](BRIEF.md)
- [Interface direction and capability map](INTERFACE.md)
- [Tool and contract handoff status](TOOL-REQUIREMENTS.md)
- [Installation and pipeline handoff journeys](USER-JOURNEYS.md)
- [UI foundation decision](docs/decisions/0001-ui-foundation.md)
- [Storage and process engineering](ENGINEERING.md)
- [Local library and host decision proposal](docs/decisions/0002-local-library-and-host.md)
- [Foundation runtime and development workflow](docs/design/foundation-runtime.md)
- [Foundation implementation plan](PLAN.md)

The earlier `work/interface-study.html` includes Pixel and cross-tool workflows and does not
represent the agreed initial scope. No frontend implementation is approved by that study.
