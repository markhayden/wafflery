# Wafflery interface direction

Current implementation summary, 2026-11-03, followed by the historical interface exploration.
The [M5 review](docs/reviews/2026-11-03-m5-operations.md) compares the implemented hierarchy and
shared drawer against the original studies. The mock interactions remain illustrative.

## Implemented workspace

The application header owns Tool; the selected tool name/purpose heads the workspace. The form is "Config": numbered steps in fill-in order (images, locations drawn on the image, required
settings, optional settings folded, spending limit last); every input has an ⓘ in plain words. The
result is one comparison with a Mask overlay switch, outputs as rows (Details opens a drawer). Pickers are dropdowns anchored to their control (a popover: search pinned, list scrolling, outside
click or Escape closes, flips above when below is short). One matching
focused picker chooses Playground, Operations or Utilities where declared, followed by a searchable,
grouped activity picker. Each workspace option says what it offers in plain words naming the tool
(Playground: "Test out complete jobs. Provide a series of inputs and see how <tool> works end to end.");
the closed control shows a compact count ("7 complete jobs"). The heading shows a known tool's longer
description and a "Learn more" link to its wafflehammock.com page; other tools show their declared
purpose. Running text never ends on a lone word (`text-wrap: pretty`; headings `balance`). Task-only tools omit unavailable workspace kinds. Both selectors use the
same control. On a desktop-width window at least 32rem tall, the input and result panes fill the window's
height exactly and each scrolls its own content between its fixed header and footer; the page itself does
not scroll. A phone-width window stacks the panes and scrolls the page, and a window shorter than 32rem
scrolls the page with the side-by-side panes ending on one line, so there is never empty page below a
shorter pane.

Tool setup lists an action's requirements for the current inputs: a requirement with `for_choice` shows
only when the inputs pick that choice (a blank input picks its default), and the readiness shown, in
setup, the action picker and Run's reason, is computed the same way. With nothing required missing it
reads "Ready" with the list behind "Show requirements (N)"; otherwise the missing ones are listed open
and the rest stay behind "Show other requirements (N)". A missing optional requirement reads
"Missing · optional" and never makes the action look unready. "Present" (found, not validated) is
explained once below the list rather than on every line.

Choose tool rows show a decorative 64 px logo on the left (48 px on narrow screens; 128 px WebP files under 16 KB), the name and right-aligned version
on the first line, then the description. Known-tool presentation comes from a small local map;
other tools use their declared purpose with the same reserved logo space. An error replaces the
description. Descriptions wrap fully without truncation or clamping, and the logo stays
top-aligned with the name. Long versions truncate with the complete value available in their title and accessible
button name. Search covers name, description and version. The selected workspace still shows the
tool's declared purpose; this presentation map does not change tool capabilities or identity.

Declared ports render image, shared-location and opaque native-reference inputs, including ordered
lists. Send to fills a compatible draft without running it. Location outputs expose public geometry
and overlays; native Masks show previews plus separately labelled resource/portability state.
History/examples preserve inputs and origin links, and opening them creates an independent draft.

The shared right drawer covers inspection, reports, pipeline handoff and invocation bundles. An
ordered chain review opens each ordinary step draft with freshly produced inputs; Stop/resume and
current paid approval remain explicit. Closing the drawer keeps owned work running. Before/after
wipe remains limited to equal dimensions/coordinates; mismatches use side-by-side comparison.
Tool report HTML is embedded on an isolated origin and is not restyled by Wafflery.

The sections below preserve the original design inventory. Capabilities listed as proposals there
are not a substitute for the current description or [integration evidence](docs/integration-sources.md).

## Established scope

- One local application, usable with either tool installed or both.
- `wafflery` opens the workshop. Each tool's `playground` command finds it, offers an
  install when absent, and launches it with that tool as context. The launchers contain no UI.
- Focus on the individual Rascal and Squint workspaces first. Pixel comes later.
- Cross-tool workflows and handoffs follow the individual experiences.
- Keep the tools' detailed result pages. Shared controls and image inspection surround them.
- Interface exploration precedes stack selection and the implementation spec.
- Lead with automatic and prompt-driven outcomes. Manual geometry and low-level parameters
  are optional refinements and debugging controls, not prerequisites for the main experience.
- When an intended interaction needs new tool support, specify it for the owning tool agent.
  Current interface gaps should inform that work rather than determine the product experience.

The layouts and activity groupings below are proposals. Capability descriptions come from
the sibling projects; they do not mean every capability is already exposed through the
shared MCP contract. No installed binaries or paid actions were run for this inventory.

## Screen structure

### Workshop home

Show installed tools and recent runs. Each tool entry explains its purpose, which activities
are ready, and any setup needed for others. A missing sibling tool can have a small discovery
entry explaining what it adds. Runnable tools take precedence.

Distinguish installed from ready. Rascal can make geometry-based masks without a segmenter
or filler. Prompt-based finding needs its configured pack; painting needs a filler. Squint
needs its judge configuration and credentials, plus any enabled tool dependencies.

Opening through `rascal playground` or `squint playground` may focus that tool directly,
while keeping the same navigation. Reusing a running Wafflery instance is proposed startup
behavior. The install mechanism and instance discovery still need specification.

### Tool workspace

The current mockups keep tool selection in the app chrome, followed by Playground,
Operations, Usage, and Runs. Playground lists complete tasks; Operations provides a searchable,
grouped catalogue for testing individual pieces. Focused picker is selected; Activity sidebar
is retired. Category strip was rejected because it does not scale.

Operations accepts typed inputs, exposes parameters, and runs only the selected operation.
Results follow the artifact type: for example, Line produces a Location shown as an overlay
plus data, while Feather accepts an existing Mask. Neither requires rerunning a complete task.
The Line and Feather previews in the mockups are illustrative, not algorithm output.

Use the same frame for both tools:

1. Tool and activity selection, with readiness relevant to the selected activity.
2. Input images, a prompt when useful, and automatic defaults for the selected activity.
3. A large result area that remains visible while controls change.
4. Run controls: estimate where applicable, run, progress, stop, terminal state.
5. Recent attempts, with open and rerun-with-changes actions.
6. An expandable inspector and the tool's detailed report.

On a wide screen, place inputs on the left and the image/report area on the right, with
supporting details in a shared right-side drawer. Pipeline handoff, run details, example
browsing, and saving examples use that shell across tools. Keep its heading, run context,
closing, scrolling, and keyboard behavior consistent. On narrow screens the drawer fills the
available width. Keep navigation compact; stack the main input/result sections when needed.
An image result gets substantial space; an empty screen explains which input is needed.

In a desktop-width window at least 32rem tall, keep the workspace within the viewport: the panes
fill its height and the page does not scroll. Input and Result scroll independently, with Run and
the saved-result actions outside their scroll areas. Keep the header compact. Phone-width and very
short windows use ordinary page scrolling so controls remain reachable at larger text sizes, and
side-by-side panes end on one line. Scroll regions are named and keyboard-focusable.

Opening an example restores input settings into a new draft. Result explicitly confirms which
example loaded, explains that Run creates a new result, and links to history for previous results.
Do not show an old result as if it came from the unexecuted draft.

Keep Quit secondary: a power icon in the header, with the tooltip and accessible name “Quit”.
It opens the existing confirmation drawer; clicking the icon alone never stops the host.
`wafflery start [--data-dir <path>] [--no-open]` offers terminal-owned hosting with startup,
request-error, tool lifecycle and prefixed tool stderr logs. It opens the browser after readiness,
prints its short-lived bootstrap link, and Ctrl-C requests orderly cancellation and child cleanup.
H1's default/tool launchers retain their detached lifecycle and quiet output. Closing a tab never
stops either mode; the same Quit confirmation remains available.
Flag conflicts with agreed interfaces before proposing changes to them.

For compatible images, offer a draggable comparison divider with a native keyboard-operable
control. Label the actual sides (Source/Result, Source/Mask overlay, or Baseline/Attempt).
Preserve aspect ratio and use the declared image frames; do not stretch mismatched images into
apparent alignment. Side-by-side saved-run comparison retains the wider workspace area.

Do not force before/after onto every activity. Rascal may produce a mask or composite;
Squint primarily produces findings about a candidate and its references. Use explicit image
roles when there is no unambiguous before/after pair.

## Rascal activity map

Lead with the outcome: repair a seam, remove an object, repair damage, or create a mask.
Run the supported automatic sequence using configured defaults. Keep the intermediate masks
and operations available for inspection. Parameters listed here are candidate controls from
the existing recipes and operations, not a fixed UI schema; most belong under advanced controls.

| Activity | Inputs | Main controls | Result to inspect | Dependency or boundary |
| --- | --- | --- | --- | --- |
| Repair a seam | Panorama and configured filler | Automatic wrap-edge repair; optional supported overrides | Repaired panorama first; seam band, prepared input, fill, composite and measures in inspection | Existing `heal seam` path; no drawn line required for a panorama wrap seam; arbitrary seam detection is not implied |
| Remove an object | Image, object prompt, configured finder and filler | What to remove; optional instance/refinement overrides | Edited image first; found regions, mask and intermediate artifacts in inspection | Existing `heal remove-prompt` path; expose it through the shared interface |
| Repair damage | Image and description of the scratch, stain, or defect | What to repair; optional region correction | Repaired image and the located region/mask | Desired prompt-led interaction; existing `repair` requires a stroke and `repair-area` a box/polygon; automatic localization needs tool work |
| Create a mask | Image and object/region prompt | What to select; optional selection/refinement overrides | Mask overlay and rendering | Existing `remove-prompt` supports object-oriented selection; broader region language requires validation or tool work |
| Refine a region manually | Image and rectangle, polygon, or stroke | Region; protection; growth, shadow extension, closing, feathering | Mask and optional edited result | Advanced route using `remove`, `repair`, or `repair-area`, with `heal` when painting |
| Protect a previous edit | Earlier composite and its mask | Previous edit to preserve | Inverted mask showing the area available for a second pass | `protect` recipe; preserve frame/provenance compatibility |
| Make a simple mask | Image and rectangle | Rectangle | Plain mask rendering | Advanced `rect` recipe; useful offline engineering fixture |
| Run a recipe directly | Recipe inputs and configured filler when painting | Recipe settings; filler; supported fill settings | Mask, prepared input, returned fill, composite, measurements | Advanced access to existing recipe and `heal` orchestration |
| Composite an existing fill | Base image, fill image, compatible mask | Supported paste mode | Composite and preservation measurements | `paste`; mode restrictions remain the tool's responsibility |
| Inspect or render a mask | Existing mask; image when needed | Grey, binary, alpha, tinted, or inspection rendering; applicable threshold | Rendering, zone statistics, provenance | `render` and `inspect`; these views serve different purposes |
| Measure a result | Before/after images, mask, optional line | Applicable measurement set | Continuity, tone step, transition, texture match, drift | `measure`; unavailable measurements need an explanation |

The main form should ask what the person wants to achieve. Internally, a finding prompt and
a painting prompt serve different purposes. If both are needed, Rascal must define and expose
their interpretation; Wafflery must not silently send the same text to both. The current
recipe runner distinguishes finder-bound prompt text from filler text. A general free-form
instruction parser is not assumed to exist.

For the first design pass, use **Repair a seam**, **Remove an object**, **Repair damage**, and
**Create a mask** as the primary activity concepts. Keep protection, direct recipe execution,
compositing, rendering, measurement, and manual drawing available as refinements or advanced
activities. A repair action promises an edited image; if only a mask can be produced, offer
an explicitly named mask-only action and explain the missing painting dependency.

Automatic does not mean a language model is required for every step. Panorama wrap-seam
repair has a known location and can start from the image and configured defaults without a
prompt. Prompt input is useful for selecting an object or expressing a repair requirement.

### Rascal advanced operations

The advanced view should make the operation catalogue discoverable without requiring it for
a normal recipe run:

- Find: `line`, `rect`, `stroke`, pack `segment`.
- Refine: `rasterize`, `tighten`, `grow`, `shrink`, `close`, `subtract`, `select`, `invert`,
  `fan`, `profile`, `feather`.
- Prepare: `cut`, `put`, `ramp`, `apply`, `window`, `unwindow`.
- Output and inspect: `render`, `paste`, `measure`, `inspect`.

Their typed artifacts and frames are richer than interchangeable image files. The host must
not pretend a mask rendering contains the whole native mask record. Artifact controls need
a declared, supported interface; the detailed tool page remains the place for internal views.

### Rascal debugging

The most valuable inspection follows a mask through the recipe: located region, selected
instances, growth, protection, feathering, prepared pixels, fill, and composite where present.
Show the selected run's resolved parameters, step outcomes, warnings, and artifact provenance
through the tool's report. A `not_applicable` operation needs its reason, not a blank result.
Do not promise an editable step timeline or resume-from-step behavior until Rascal exposes it.

## Squint activity map

Squint's five use cases are proposed entries in its activity picker. It remains responsible
for resolving rules, running its judge, and computing the verdict.

| Activity | Inputs to present | Main controls | Result to inspect |
| --- | --- | --- | --- |
| Check an edit | Candidate, source, brief/requirements | Source-fidelity rules and permitted changes where supported | Findings tied to candidate/source; preserved and violated obligations |
| Check a reference subject | Candidate, labeled references, brief/requirements | Reference-consistency rules; what identifies the subject | Corresponding evidence and findings about the subject |
| Check realism | Candidate and any relevant requirements | Reality-check rules | Anatomical, physical, text, or other realism findings |
| Check a brief | Candidate and requested content | Brief-compliance rules and requirements | Item-by-item obligations and findings |
| Check a product | Candidate, relevant product source/references, requirements | Product-fidelity rules; identity details to preserve | Findings about labels, logos, shape, colour, and proportions |

Actual required image slots and argument shapes come from the tool's description. The table
describes user intent; it must not become hardcoded dispatch logic or inferred validation.

### Squint setup controls

The desired controls are judge/provider configuration, rule profile, additional requirements,
enabled evidence tools, and run limits. Keep normal brief entry prominent and detailed rule
or budget settings in an advanced section. Show the resolved configuration in the report.

These are not all available as generic selectors today. The current external-interface plan
keeps `rules`, `tools list`, and `catalog` on the CLI under tool-specific description links.
A generic host needs declared choices and supported arguments/resources before offering model,
rule, or evidence-tool pickers. It must not parse Squint configuration or shell out to those
commands as an undocumented fallback.

### Squint debugging

Start with the findings and the relevant images. Let the user inspect obligations and their
dispositions, cited evidence, locations, model-visible images, tool calls/results, retries,
repairs, usage, and cost through Squint's report. A missing object may have no geometry;
do not invent a bounding box for it.

Keep execution status separate from the judgment: a completed check can judge an image
`fail` or `unsure`. The common header contains execution status; the verdict is tool-owned
data. Without a shared semantic declaration, the host must leave verdict presentation to
Squint's report rather than extracting fields from its body.

## Shared behavior and ownership

| Interface responsibility | Owner and rule |
| --- | --- |
| Tool name, actions, forms, image/location inputs | Host reads the contract description and schemas |
| Action behavior and validation | Tool; host form validation supplements it |
| Input selection, uploads, accessible folders | Host passes explicit input directory grants; tool config may narrow them. Evidence-read folders are separate |
| Progress and stop | Host displays notifications and sends cancellation; terminal state is reconciled through run resources |
| Cost | Host displays declared cost/estimate data; tool enforces its limits |
| Common run facts | Host shows status, times, spend, cache, notes, errors, images, and contract locations |
| Detailed journal meaning, internal frames, specialized measures, verdict | Tool report; journal content has no common event schema |
| History and rerun | Tool resources supply runs; host records submitted arguments and owned input snapshots for its own invocations, sharing identical stored files |
| Image comparison | Host can compare explicit images; specialized mask charts and panorama report behavior stay with the tool |
| Finding labels | Host uses supplied vocabulary without translating it into invented scores |

Changing controls creates a new draft. Running it creates a new attempt; it does not rewrite
the previous result. A rerun may be cached and must say so. History imported from a tool may
be viewable without enough information to rerun. Missing or changed input files need an
explicit state instead of silently substituting the current file.

## Interface gaps to settle before the build spec

1. **High-level actions.** Rascal's current server exposes registered operations, pack tools,
   and workspace helpers. Recipes, `heal`, and lab orchestration are not automatically part
   of that interface. Rascal should expose the selected activities as conforming actions;
   Wafflery should not reconstruct Rascal recipes.
2. **Squint action naming.** Its plan describes one action per use case, while its server
   description also names a single `check` tool. Align the emitted description and MCP tools
   so the host can invoke exactly what it discovers.
3. **Useful forms.** Agree on how schemas express labels, help, defaults, choices, advanced
   settings, and conditional inputs. The host can render JSON Schema, but task-oriented
   grouping and live choices are not guaranteed by v0.1.
4. **Automatic input and optional correction.** Tool actions should locate and process regions
   from prompts or known task geometry where supported. Prompt-led damage localization needs
   additional Rascal support. Manual rectangle/polygon/stroke/line editors are optional
   refinements; output report embedding alone does not provide these input controls.
5. **Readiness and available configurations.** v0.1 declares needed environment-variable names,
   but does not standardize pack/filler availability, configuration choices, or setup remedies.
   Do not mark a whole tool unavailable because one optional activity is unavailable.
6. **Reports.** Define render URL readiness and process lifetime, including browser framing
   compatibility. Limit the first version to local runs with usable run directories.
7. **Estimates and budgets.** Define the shared estimate request/result and how remaining
   budget is shown for a long-lived server. Restarting must not silently bypass spending limits.
8. **History.** Owned input snapshots with file deduplication and retention until explicit
   deletion are agreed. Show disk usage and cleanup controls. Decide storage location,
   output/report preservation, and effective configuration capture. Fingerprints
   verify identity but do not recover deleted files or every original argument.

Treat these as proposals for the relevant tool or table-manners repository. No sibling files
have been changed by this interface pass. [Tool requirements](TOOL-REQUIREMENTS.md) separates
the initial handoff proposals by owner and records their acceptance criteria.

## Evaluations and later scope

Single-tool evaluation remains an intended use of Wafflery. Rascal already has corpus and
matrix runs, blind pair judging, and decoded outcomes. Squint has evaluation matrices, arms,
datasets, estimates, and gates. Those execution and review surfaces need their own contract
mapping; they are not yet justified as working buttons in the first interface.

For now, design a complete individual run and review experience before adding batch evaluation
screens. Pixel, cross-tool workflow execution, automatic repair loops, a node editor, hosted
accounts, and motion-specific interfaces are outside this first design pass. Their absence
does not prevent either individual tool from being useful.

## First walkthroughs to validate

1. **Rascal seam repair:** select Repair a seam, supply a panorama, use a configured filler,
   run without drawing a line, and inspect the repaired panorama and intermediate mask.
   Change a supported setting and compare attempts. No Squint or Pixel required.
2. **Rascal object removal:** provide an image and an object prompt, run with a configured
   finder and filler, then inspect the edit and how the object was located and masked.
   With no filler, clearly offer Create a mask instead of reporting a completed removal.
3. **Squint alone:** choose source fidelity, supply source/candidate and requirements, inspect
   the cost estimate, run, and follow a finding into its evidence and full report.
4. **Interrupted Squint run:** stop during progress, recover its terminal state and spend, then
   open it from history. A cancelled check must never appear as a passing image.

These walkthroughs test the main assumption: the same application frame can feel natural for
automatic image editing and image judgment while the tools retain their own behavior and
reports. A geometry-only mask run remains a useful offline fixture, not the primary product
onboarding flow.

## Sources

- [Playground brief](BRIEF.md)
- [Shared contract](../table-manners/CONTRACT.md)
- [Rascal recipes](../rascal/docs/recipes.md)
- [Rascal operation catalogue and lab specification](../rascal/SPEC.md)
- [Rascal current server entry point](../rascal/crates/rascal-mask/src/serve.rs)
- [Squint jobs, reports, and external interface](docs/integration-sources.md#squint)
- [Squint M7 interface plan](docs/integration-sources.md#squint)

The earlier `work/interface-study.html` explored Pixel and cross-tool workflows. It predates
the narrowed scope and is not the proposed initial navigation.
