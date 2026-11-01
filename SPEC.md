# Wafflery specification

Status: product acceptance consolidated, engineering specification still in progress. Started
2026-10-30 through the kickoff workflow. The reviewed interface and decisions Q1–Q19 now have
[testable acceptance criteria](ACCEPTANCE.md) and [owner handoffs](TOOL-REQUIREMENTS.md). This is
not yet an approved implementation spec or build plan.

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
  must be labeled. The current mock downloads a manifest only; the bundle is not implemented.
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
- **Q9, confirmed:** Show paid-run estimates and remaining tool session budget beside Run.
  Clicking Run authorizes that attempt within the displayed/enforced cap; no second routine
  confirmation step. Changed inputs require a refreshed estimate. Results show reported spend
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

## Acceptance and remaining planning gates

[ACCEPTANCE.md](ACCEPTANCE.md) is the testable product checklist, AC01–AC22. Its gates G1–G6
identify unresolved engineering work: output/report durability; budget/process lifecycle;
discovery/install/host reuse; storage/roots/cleanup; current action coverage; and the engineering
stack/commands/test/package details. The product choices in the decision log stay settled.

[TOOL-REQUIREMENTS.md](TOOL-REQUIREMENTS.md) indexes three owner packets with current source
observations and verification requests. The [source inventory](docs/integration-sources.md)
records Rascal M8 merged/tagged and Squint's locally merged server work, with its release still
unconfirmed. These are source observations, not newly executed conformance tests.

Routine implementation recommendations are explicitly labeled in the acceptance gates; resolve
material choices one at a time and verify existing code before asking the owner. Under Q19, close
the engineering choices applicable to a milestone before locking its implementation/commit plan.
Open C1–C4/H1 agreements gate their integrations, not unrelated foundation tasks.

## Engineering sections to complete

[ENGINEERING.md](ENGINEERING.md) now proposes storage ownership, capture/deletion ordering,
host/tool/viewer lifetime, minimal interruption handling, and file-access boundaries.
[ADR-0002](docs/decisions/0002-local-library-and-host.md) records the accepted embedded SQLite
choice (Q18) and proposed asset-store/single-host mechanics. Detailed engineering proposals
remain under review and are not implemented. C2/C3/H1 still need agreed tool interfaces.

An owner-run external CLI review found association, preservation, relocation, and interface
ambiguities. The [review disposition](docs/reviews/2026-10-30-spec-review-disposition.md) records
the corrections and qualified recommendations. The subsequent [plan review disposition](docs/reviews/2026-10-31-plan-review-disposition.md) updates source facts, task ordering, and the early H1/C4 proposals. Neither review closes an unaccepted contract gate.

[Foundation runtime](docs/design/foundation-runtime.md) now selects the initial runtime,
dependency baseline, repository layout, browser boundary, developer commands and test approach.
[PLAN.md](PLAN.md) proposes M0–M3 as the reviewable foundation implementation scope, with later
feature gates separated. Dependency installation/compatibility checks are an explicit M0 task;
listed commands are planned, not yet runnable. Full release G1–G6 remain tracked separately.

Before spec review, add the chosen architecture and stack, repository structure, executable
development/build/test commands, code conventions, persistence model, process and interface
contracts, failure states, security boundaries for local execution, and measurable acceptance
criteria. Do not invent command names or dependency versions before selecting the stack.

Verification must cover offline fake-tool integration, real tool conformance, browser
interactions and layout, cancellation/recovery, persisted runs and examples, and packaging.
Any live paid verification must have explicit scope and cost authorization.

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
- [Draft tool and contract handoffs](TOOL-REQUIREMENTS.md)
- [Installation and pipeline handoff journeys](USER-JOURNEYS.md)
- [UI foundation decision](docs/decisions/0001-ui-foundation.md)
- [Storage and process engineering draft](ENGINEERING.md)
- [Local library and host decision proposal](docs/decisions/0002-local-library-and-host.md)
- [Foundation runtime and development workflow](docs/design/foundation-runtime.md)
- [Foundation implementation plan](PLAN.md)

The earlier `work/interface-study.html` includes Pixel and cross-tool workflows and does not
represent the agreed initial scope. No frontend implementation is approved by that study.
