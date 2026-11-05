# Installation, exploration, and pipeline handoff

Status: current host behavior through M5.21 is documented in [README](README.md) and the
[M5 review](docs/reviews/2026-11-03-m5-operations.md). The original installation/mock walkthroughs
below preserve the design intent; a real installer/distribution remains later work. Rascal and
Squint work independently. Guided per-step chain review now exists; unattended orchestration,
Pixel and batch evaluations remain later scope.

## Outcome

A user should leave with confidence in a tested configuration and a concrete way to use it
outside Wafflery. Inspection and reproducible pipeline handoff are first-class outcomes.
Reproducible means preserving the invocation, inputs, settings, and relevant versions; it
does not promise identical outputs from a nondeterministic provider.

## Reviewable mock

The [local mockup](work/mockups.html) now connects these journeys. Its Start from selector
offers Wafflery first, Rascal first, Squint first, and a ready workspace. Changing that
selector resets the simulated installations, configuration, and in-memory run history.

The browser walkthrough begins after Wafflery has started. Tool-first paths open directly
in the requested tool; any missing-host installation offer happens in the terminal before the
browser opens. The walkthrough includes an empty host and per-tool setup,
independent Line/Feather runs, inline provider estimates, progress/cancellation, immutable run
snapshots, reopening history, and the Use in your pipeline pane. Copy and manifest download
work locally. The manifest excludes assets and is explicitly not a portable executable bundle.
Only Line includes a concrete CLI example; other command and MCP exports remain labeled
contract gaps. Setup, findings, images, and execution are illustrative, never live tool output.

DOM interaction checks cover the three journeys and snapshot/export behavior. Browser layout
verification remains unavailable in this authoring environment.

## Common destination

All three entry points open the same local host and browser interface. Release binaries include
the built UI, so using Wafflery requires neither Node nor a frontend build. Tool executables,
models, provider access, and other action dependencies remain separate installations.

The host discovers supported installed tools, reports compatibility, and evaluates readiness
per action. A missing dependency disables only actions that require it. Another tool can be
discovered without becoming a prerequisite. Installation never happens merely by selecting a
tool. Exact distribution channels, installation prompts, and update behavior remain to specify.

## Rascal first

User story: I have Rascal and want to understand what its operations do, then use a tested
configuration in my own pipeline.

1. Install Rascal using its supported release distribution, or use an existing installation.
2. Run the proposed `rascal playground` launcher. If Wafflery is absent, receive a clear
   terminal installation offer. After confirmation, the launcher installs/starts the shared
   host and opens the browser directly in Rascal. No host-install screen exists inside the host.
3. Land in Rascal's workspace, with task readiness visible. Existing configuration is reused.
   With no segmenter or filler configured, supported geometry operations remain available.
4. Choose a complete task such as Create a mask when its dependencies are ready, or choose
   Operations → Line for a simple independent experiment. Choose an image and provide the
   relevant prompt or parameters. The UI explains any missing configuration for this action.
5. Run and inspect the actual output type. Line returns geometry shown as an overlay and data;
   it does not find a seam or paint pixels. Mask-producing actions show the mask and its inputs.
6. Adjust a parameter and run again. Earlier attempts retain their own inputs and settings.
   Select a compatible saved artifact to test a subsequent operation independently when the
   tool exposes that binding.
7. Open Use in your pipeline on the chosen run. Copy a supported invocation or export a
   reproducible bundle with its required input artifacts. Integrate it without Wafflery
   being a runtime dependency.
8. Squint is discoverable as an optional way to inspect image quality. Installing it adds
   another independent workspace; automatic handoff is deferred.

Acceptance: Rascal is useful without Squint. Missing paid providers do not block supported
local operations. Export describes the selected run, not subsequently edited form values.

## Squint first

User story: I have Squint and want to understand its judgments and find a configuration I
can call from an image pipeline.

1. Install Squint or use an existing installation. Launch the proposed `squint playground`.
   The same terminal installation offer applies if the host is missing. After installation
   and startup, the browser opens directly in Squint.
2. Land in Squint. See its configuration/readiness, including the selected judge and missing
   environment-variable names. Wafflery does not store provider keys.
3. Choose a supported check. Supply the candidate image, references where relevant, and the
   brief/rules the check requires. Optional dependencies affect only checks that need them.
4. Run. For a paid action, optionally set a spending limit or check the cost first; without
   either, the tool's own spending limits apply. No additional confirmation is needed; a check
   made before an edit no longer binds the run. Inspect live progress; cancellation
   reports the resulting state and spend rather than claiming a refund.
5. Review findings, evidence, locations, and the tool's own detailed report. A failed or
   interrupted run remains inspectable. No direct access to internal evidence tools is assumed
   unless Squint advertises those operations.
6. Change a rule or setting, rerun, and inspect the new run. General Usage explains the API;
   Use in your pipeline provides the chosen run's concrete invocation and dependencies.
7. Discover Rascal as an optional mask/repair tool. Installing it exposes its independent
   workspace without changing Squint's working configuration.

Acceptance: no Rascal installation required; missing judge configuration has an actionable
setup state; no surprise paid runs; findings and invocation information remain accessible.

## Wafflery first

User story: I want to explore these tools visually before choosing which ones belong in my
pipeline.

1. Install the Wafflery release binary and start `wafflery`.
2. If compatible tools are already installed, show them and their readiness. If none are
   found, show a useful start screen explaining Rascal and Squint, with setup instructions.
   Avoid an empty form and avoid making the user install the entire pipeline.
3. Choose the tool that matches the intended job. First-release proposal: show its supported
   install command/link, allow the user to install it, then explicitly recheck discovery.
   A managed in-app installer is a separate decision, not an assumed requirement.
4. Enter the newly available workspace. Complete only the configuration needed for the chosen
   action. Distinguish missing executable, incompatible version, and missing model/provider.
5. Use an example or local input, run an action, inspect it, refine it, then export the tested
   configuration through Use in your pipeline.
6. Add the other tool later from the same discovery/setup surface. Existing runs remain intact.

Acceptance: no-tools-installed is a designed state; installing one tool is sufficient; newly
installed tools can be detected without reinstalling Wafflery; setup shows concrete remedies.

## Returning user

Open the host directly or through either tool's launcher. Proposed behavior is to reuse the
existing host where compatible and focus the requested tool.

The owner confirmed that closing a browser tab leaves the local host and active runs running.
Reopening reconnects to current run state without resubmitting work. The interface should make
that behavior clear while a run is active. Stop run requests cancellation of one run; Quit
Wafflery shuts down the local host and requires confirmation if work remains active.
Rejecting that confirmation leaves the host and runs alone. A confirmed quit must use graceful
cancellation and preserve terminal state and spend where the tools support it.

This does not promise survival across a host crash, computer restart, or sleep. On restart,
restore persisted history and mark attempts without a recorded terminal outcome as interrupted.
Keep saved evidence; show unknown outcome or spend honestly. Never automatically retry or resume
computation. Explicit retry creates a new attempt. Durable job recovery is outside the first
release because these are exploratory runs. Process detachment and instance discovery remain
technical design work; tab closure must not accidentally trigger shutdown.

Runs reopen with their original configuration and results. A rerun creates a new run.
Unavailable tools, incompatible versions, and missing managed assets are explicit states.
One personal local library holds history and examples across projects and all three entry
paths. It defaults to Wafflery's application-data folder, without requiring a project.
Settings show its location and disk usage and allow choosing another storage location.
The proposed relocation moves the owned history/image library on this machine. Tool-native
workspace storage stays at a separate stable location, displayed with its own disk usage;
moving it requires tool support. This is not a portable backup of the entire tool installation.
Exact platform paths, relocation behavior, and output/report preservation mechanisms remain open.

### Accepted input preservation

For runs started through Wafflery, preserve owned copies of required input files before
execution. Execute against those snapshots so history and exports refer to the bytes actually
used. Moving, editing, or deleting the user's original file must not affect the saved input.
Keep the original path as provenance, not as the only way to retrieve the file.

Store byte-identical files once and let runs reference the shared stored copy. The storage
digest verifies the retained bytes; it must not be conflated with a tool's potentially
normalized image identity. Snapshots must not share mutable file contents with originals.
Retain supported typed artifacts through their declared persistence mechanism; an unsupported
artifact remains an explicit reproducibility gap, not a reason to substitute a rendered image.

If required inputs cannot be preserved, explain the problem before starting the run. A future
cleanup must not remove an asset still referenced by another retained run or saved example.
Importing prior CLI/agent history is deferred. If added later, do not claim those runs have
snapshots until their required assets have actually been obtained and verified.

Acceptance: run with an image, move or edit the original, then reopen and export the run using
the preserved input. A second run using identical bytes shares the stored asset. The prototype's
in-memory history does not yet implement this storage behavior.

### Accepted output preservation

Preserve tool-declared outputs and debugging artifacts alongside the inputs and settings:
intermediate masks, geometry, findings, and data required to inspect reports. Saved review must
survive removal of disposable tool files. Exclude disposable caches and model downloads, and
label unsupported preservation explicitly. Tool/host ownership and report reconstruction versus
snapshotting remain engineering work; this requirement does not authorize guessing file layouts.

### Accepted retention and cleanup

Keep history until the user explicitly deletes it. The first release has no automatic expiry
or silent eviction to make room. Show storage usage and provide explicit cleanup controls.
Automatic retention periods may be considered later; they are outside the first release.
Distinguish total stored bytes from bytes a selected cleanup can actually reclaim, because
multiple runs or saved examples can reference the same retained asset.

Deleting a run removes its history entry and its ownership references. Delete managed assets
only when no retained run or saved example needs them. Never delete the user's original files
as part of history cleanup. This does not authorize deleting tool-owned run directories; their
ownership and cleanup interface remain to specify.

If storage is exhausted, explain the issue and offer cleanup or a storage-location change;
do not silently sacrifice retained history. Detailed cleanup interaction remains open. These
are persistence requirements; the mock still uses session-only demo history.

## Examples and Runs

The owner confirmed two separate concepts: Runs records attempts and results; Examples holds
named reusable starting points. Offer bundled starters and Saved by you together through Open
example in the tool workspace. A bundled starter explains its purpose and prerequisites.

Save as example captures the selected completed run's inputs and settings, with a name and
optional notes. It never substitutes untested current-form edits. Opening an example fills a
draft, shows its readiness, and does not invoke a tool or present the original output as a new
result. Run creates a new attempt; editing a loaded draft does not modify the saved example.

Saved examples own references to their required preserved inputs/artifacts. Deleting the source
run must not break an example. The mock stores examples in memory, scoped by tool, and resets
them when the starting point changes; real storage follows the agreed explicit-deletion policy.

Acceptance: complete A, edit the form, save A under a name, and reopen that example. The form
contains A's saved parameters, no execution occurs, and no new run exists until Run is clicked.
Running a modified draft creates B without overwriting A or the example.

## Compare attempts

The mock offers side-by-side and slider modes for compatible visual outputs. A movable divider
reveals the baseline on one side and selected attempt on the other; a native range control
supports keyboard adjustment. The ordinary result viewer also offers Source/Mask overlay or
Source/Location overlay for the illustrative examples. Real Source/Result comparison requires
actual compatible image outputs. Mock overlays must not be presented as real repaired images.

The owner approved pinning a saved run as a baseline for manual comparison. In the current
mock, complete an attempt, choose Pin this run, change a setting and run again, then choose
Compare. Show baseline and selected attempt side by side with saved settings, changed fields,
status, and labeled spend. Baselines are scoped to the same tool task/operation; switching
tasks does not silently compare unrelated outputs. Clear removes the pin, not either run.

Comparison uses immutable saved snapshots, not current form values. Image/mask/location outputs
use visual previews; judgments include findings/evidence. The mock demonstrates Line geometry,
illustrative masks, and sample Squint findings. It does not calculate quality scores, declare
a winner, or run a batch evaluation. In the real application, identify changed input files and
reference frames explicitly; overlay alignment must not imply that different inputs are identical.

Acceptance: pin run A, change parameters and create B, then compare A/B. Editing the form to C
without running it does not change either side. Reopening A from history does not erase the pin.
An unrelated task cannot silently use that baseline. Pins should persist with real history;
the mock clears all state when the starting-point selector changes.

## Use in your pipeline

Open this view in the shared right-side drawer, with the selected run in its header. Run
details, browsing examples, and saving an example use the same drawer shell. Opening these
views must not grow the main page or change the selected run. Closing restores keyboard focus
to its trigger; Escape and the close control dismiss it. Use full available width on narrow
screens and honor reduced motion for the slide transition. The current prototype uses a native
modal dialog; real focus and browser layout verification remain part of implementation checks.

General Usage and run-specific handoff have different responsibilities:

- Usage explains what an action accepts and returns, with tool-owned CLI/MCP documentation.
- Use in your pipeline describes how to reproduce the selected attempt using its immutable
  input/configuration snapshot. A draft form may have its own preview, explicitly labeled
  untested. It must never be presented as the invocation that produced an older result.

Proposed pane contents:

- Supported CLI invocation with safe platform-appropriate quoting.
- MCP call payload and the connection/setup information needed to use it; a JSON payload alone
  is not a complete standalone client. The first-release requirement is recorded call/setup data
  for an MCP client; generated client programs are optional and require independent replay tests.
- Effective configuration, tool version, declared model/provider identifiers, and any seed
  actually exposed by the tool. Unavailable reproducibility metadata is identified explicitly.
- Input manifest: image roles, file fingerprints, references, and required typed artifacts.
  A typed Mask must not silently be replaced with a rendered PNG.
- Prerequisites and environment-variable names, never credential values.
- Copy actions and a Download example bundle containing the supported invocation,
  configuration, manifest, and required input assets by default (owner confirmed). Export format remains
  to be selected; shell, Python, or TypeScript must not be offered without a verified execution
  path and supported client dependencies.

For independent operations, exporting only the final call may be insufficient: a Feather run
can require a persistent Mask artifact from an earlier operation. The export must either carry
that artifact using a supported import mechanism or include a supported reconstruction recipe.
If neither exists, state that it depends on the existing local workspace; do not call it portable.

The host can record the actual outgoing MCP call: action names, schemas, and stdio startup are
already defined by contract sections 3 and 5. CLI mapping and resolved configuration require
additional tool-provided information. An input digest identifies a configuration but cannot
reconstruct it. The host must not derive CLI flags from display labels, invent SDK methods,
or rewrite tool algorithms into generated source. Exporting one tool's supported recipe is
distinct from building a cross-tool workflow, which remains deferred.

### Export review: accepted asset default

Two actions serve different needs: Copy command/call reuses the current environment; Download
example packages the reproducible handoff. The pane should explicitly distinguish local paths
and saved artifacts from an example that includes its inputs. The owner confirmed including
required images and supported artifacts by default, with a file list and an option to reference
existing files. The following requirements guide the export implementation:

- Copy uses the selected run's immutable snapshot and declares workspace dependencies.
- Download includes supported invocation/setup, configuration, a manifest, and required input
  files by default, with an explicit option to reference existing files instead. Show included
  files and size before downloading. Never include credentials or tool/model installations.
- If a required typed artifact cannot be exported or reconstructed, label the example as
  workspace-dependent. A manifest alone is not a runnable or portable example.
- Record versions and exposed randomness controls; do not promise identical provider outputs.

Acceptance: with the declared prerequisites and inputs present, a copied/exported example can
invoke the same supported action outside the playground. Paid invocations stay explicit, and
validation of the exporter starts with the free conforming test tool.

## Requirements exposed by these journeys

Paid information comes through tool/provider integrations: estimates from the configured
provider adapter and actual spend where available, otherwise labeled estimated spend. Credential
presence is not proof of working access or a source of pricing/account balance. Provider balance
must not be confused with the tool's configured session budget. Missing configuration, failed
authentication, and unavailable estimates need explicit states; unknown cost is not zero.

1. Tool discovery, compatibility, readiness per action, and a recheck mechanism.
2. One host installation shared by both launchers; instance reuse and process lifecycle that
   implements the accepted tab-close, reconnect, cancellation, and confirmed-quit behavior.
3. File import/allowed roots and stable artifact selection across runs.
4. Immutable run-input/configuration snapshots and provenance metadata.
5. Tool-supplied invocation templates or export actions, plus portable typed-artifact handling.
6. Working tool report embedding and supported render lifecycle/address reporting.
7. Estimates, cancellation, remaining spend, and budget scope for the persistent host.
8. Offline setup examples clearly distinguished from actual tool output, if included.

These are specification inputs and handoff candidates, not evidence that the current contract
already supports every behavior.
