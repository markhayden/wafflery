# Wafflery first-release acceptance criteria

Status: product acceptance targets with implementation evidence through M5.21, updated 2026-11-03.
The criteria remain release targets; passing host integration does not approve M7 distribution.
See [M5 review](docs/reviews/2026-11-03-m5-operations.md) for actual coverage and exclusions.

## Scope and evidence

The first release makes Rascal and Squint independently useful for individual experiments,
inspection, manual comparison, history, examples, and pipeline handoff. It includes the full
set of **supported, advertised** actions; mock labels are not a list of already shipped tool
capabilities. Pixel, cross-tool orchestration, batch evaluations, hosting, and a node editor
remain deferred. Prompt-led damage localization is an optional Rascal capability request.

Use a conforming fake tool for offline integration, tool-owned controlled providers for real
adapter tests, browser tests for interactions, and installed-binary checks for distribution.
Mock DOM checks prove prototype wiring only. They do not prove native dialog behavior, layout,
accessibility, MCP conformance, persistence, payment enforcement, or executable exports.

The criteria below are product targets; coverage and remaining release gates are recorded separately. Handoff references
are defined in [TOOL-REQUIREMENTS.md](TOOL-REQUIREMENTS.md).

## Entry and setup

### AC01 — Three entry points converge

- Given Rascal alone or Squint alone, its `playground` launcher opens the shared host with that
  tool selected. The sibling tool is not required.
- If the host is absent, installation is offered in the terminal. A no leaves it uninstalled;
  a yes follows the agreed install path, then launches. Failure does not open a broken page.
- Wafflery serves no installation screen before it exists. When started directly with no
  tools, its first browser screen explains the available tools and their setup paths.
- Running an installed release requires neither Node nor a frontend development server.

Verify with isolated executable search paths and a fake installer/launcher. See H1 and AC21.

### AC02 — Discovery and compatibility are actionable

- Distinguish not installed, incompatible, installed but not configured, and ready.
- After installation or configuration changes, recheck discovers the new state without
  reinstalling Wafflery or losing history. Optional missing tools remain discoverable.
- Discovery recheck does not silently restart an active tool. Applying configuration that needs
  restart explicitly shows the session/budget boundary and waits for idle or an explicit stop.
- Unknown/incompatible declarations fail visibly; do not silently use a CLI fallback or infer
  a tool's semantics from its executable name.

Verify with discovery fixtures and malformed/version-incompatible descriptions. See C1/H1.

### AC03 — Action and form discovery stays generic

- Playground contains complete tasks; Operations contains independently callable pieces in a
  searchable, grouped catalogue. Both selectors use the same component and dimensions.
- Every supported advertised action remains reachable. Utility actions such as estimate are
  classified using agreed metadata, not hardcoded lists of Rascal/Squint action names.
- Form fields, required roles, defaults, allowed choices, and validation follow declarations.
  Unknown wire fields are not invented to make a mock control work.
- A differently named fake tool with equivalent declarations produces the same host behavior.

Verify schema-driven fixtures, keyboard picker behavior, and catalogue searches. See C1/R3/S1.

### AC04 — Readiness belongs to the action

- An absent filler prevents tasks that require painting, while supported local operations and
  mask-only tasks remain available with their own prerequisites.
- Required provider/model configuration and missing credential-variable names have actionable
  setup states. A present key alone does not establish authenticated provider readiness.
- Tool configuration owns secrets and provider defaults. Wafflery neither stores keys nor
  includes them in history, examples, copied code, logs shown to users, or downloads.

Verify absent, partial, configured, and provider-authentication-failure fixtures. See C1/R1/S1.

## Run and inspect

### AC05 — Independent pieces run independently

- With an image, position, and width, Line returns a Location and a correctly located overlay
  plus raw data. It defines geometry; it does not claim to detect a seam or paint pixels.
- With a compatible saved Mask, Feather accepts a changed parameter and runs without invoking
  a finder or filler. Geometry can separately feed Rasterize through a supported binding.
- Reject type/frame/identity mismatches clearly. Explain missing artifacts without silently
  rerunning a whole task. Do not replace a native Mask record with its rendered PNG.

Verify against Rascal's independent client and controlled fixtures. See R3/C1/C2.

### AC06 — Task outcomes mean what their labels promise

- Repair a seam takes a panorama and configured filler, with no required prompt or manual line
  for its known wrap boundary. Automatic arbitrary-seam detection is not implied.
- Remove an object uses the declared finder/filler path and returns a composite when successful.
  Create a mask makes no filler call and presents a mask rather than an edited image.
- A mask-only result cannot silently satisfy an interface action promising a repaired image.
- Unsupported automatic damage localization remains explicitly unavailable until the owning
  tool provides it; this does not block supported seam/removal/mask workflows.

Verify controlled panorama, finder, filler, and missing-dependency fixtures. See R1/R2/C1.

### AC07 — Squint results retain their meaning

- The described action is callable under exactly its advertised MCP name and schema.
- Candidate/source/reference inputs and rules/brief requirements follow the chosen use case.
- Findings, evidence, locations, confidence words, and verdict retain the tool's vocabulary.
  A completed check that judges an image unsuitable is not an execution failure.
- Standalone evidence tools appear only when advertised as independently callable operations.

Verify scripted judge outputs, no-match cases, and execution errors separately. See S1/C1.

### AC08 — One click, optional spend limit

- Run works for a paid action with no estimate and no cap: the tool's own limits apply. An
  optional spending limit lowers the tool's cap; an optional Check cost shows the worst case, the
  limit in force and the remaining **tool session** budget, and binds the run to that price.
- Editing cost-relevant inputs, model, configuration, or budget makes a check stale; Run then
  sends the call unbound. A stale response never binds a newer draft. Unknown cost is not
  displayed as zero.
- The tool enforces the submitted cap within configured limits. Exhausted budgets prevent paid
  calls. Multiple tabs do not independently reserve the same available allowance.
- Show reported spend and its measured/estimated basis, including interrupted attempts. Account
  credit is not presented as session budget. Local operations label provider cost appropriately.
- Estimates are not user attempts/history entries. While a tool is busy, new draft estimates
  are unavailable rather than queued; refreshing them never starts another execution.

Verify stale-estimate responses and budget/cache/cancellation fixtures offline. See C1/C4/R1/S1.

### AC09 — Progress, stop, and errors preserve evidence

- Show determinate progress only when a total is provided; show meaningful activity otherwise.
  Surface stale/disconnected progress rather than leaving an unexplained perpetual spinner.
- Stop requests cancellation; show stopping until the tool supplies its terminal state.
  Cancellation does not imply an immediate stop to an already submitted provider request.
- Reconcile from run resources after a dropped connection; never blindly submit another paid
  call. Terminal state, spend, error information, and completed salvageable outputs remain visible.

Verify slow, failed, cancelled, cache-hit, and disconnected fake runs. See C4/R1/S1.

### AC10 — Tabs do not own execution

- Closing a tab leaves the local host and runs active. Reopening reconnects without duplication.
- Stop run and Quit Wafflery are distinct. Quit with active work requires confirmation;
  declining it leaves execution unchanged. Confirmed quit coordinates cancellation and shutdown.
- Interrupted host/tool processes are reported honestly; browser reconnection is not a promise
  of resuming computation across a crash or computer restart.
- After host restart, restore saved history and flag attempts lacking a persisted terminal
  outcome as interrupted. Keep saved evidence and distinguish unknown outcome/spend from
  confirmed failure or zero cost. No automatic retry or resumption; explicit retry is a new run.
- Durable job recovery and exhaustive reconciliation of interrupted work are outside the first
  release. A live host still supports ordinary browser reconnection without resubmission.

Verify browser close/reopen, multiple tabs, quit decline/accept, and process interruption. See H1/C4.
Also verify cancellation before first progress: persist request/progress-token correlation,
never guess the tool run by action/time, and retain unknown outcome when association is absent.
Pre-run protocol rejection is distinct from a tool run failure and transport uncertainty.

## Preserve and reuse

### AC11 — Saved inputs survive original-file changes

- Preserve owned input snapshots before execution and execute against those snapshots. Verify
  retained bytes; do not silently run a different file if an original changes during capture.
- Store byte-identical files once across runs. Preserve original paths as provenance, not as
  the sole means of retrieval. Store the invocation and available effective configuration.
- Moving, changing, or deleting an original input leaves a managed run's saved inputs intact.
  Reopening and rerunning uses those preserved inputs and creates a new attempt.
- First-release history covers Wafflery-started attempts; importing prior CLI/agent runs is
  deferred. Native typed artifacts need supported preservation; missing assets are explicit.
- Retain tool-declared outputs and debugging artifacts, including intermediate masks, geometry,
  findings, and report dependencies. Exclude disposable caches and model downloads. Unsupported
  preservation is explicit; a rendered image cannot replace a required native artifact.
- Track saved contract evidence separately from native dependencies and report data. In v0.1,
  evidence comprises terminal result, opaque journal, verified output image bytes, and location
  data with resolvable referenced images. Unknown/unlocatable mask bytes remain missing; a
  complete evidence set alone does not claim full native/report preservation.

Verify byte snapshots, deduplication, changed/deleted originals, restart, and unavailable assets.
Remove disposable tool files and verify that supported saved artifacts remain inspectable.
M5.21 verifies effective invocation and declared native portability under C2/R3. Nonportable types and tool-owned report sources retain their explicit G1 dependencies.

### AC12 — Retention is deliberate

- Use one personal local library across projects and all three launch paths. Default to the
  application's local data folder; expose its location and a storage-location setting.
- Keep history until explicit deletion. Show total disk usage and reclaimable bytes for a
  cleanup selection; no automatic expiry or silent eviction when storage is exhausted.
- Cleanup leaves originals and assets referenced by retained runs or examples intact.
  Deleting host history does not imply permission to delete tool-owned run directories.
- Storage errors explain what failed and how to recover; they do not claim a run was saved.

Verify shared history across launch paths, shared-asset reference counts, saved examples,
storage-full/write failures, and restart. Specify relocation before implementing its setting.

### AC13 — Examples are reusable drafts

- Open example exposes bundled starters and named examples saved by the user, separate from Runs.
- Save as example takes the selected completed run's settings and required assets; current draft
  edits are excluded. Names and optional notes belong to the example.
- Opening one fills a draft without a tool call or a new result. Running the draft creates a new
  attempt; subsequent edits leave the example unchanged.
- Deleting its source run does not break a retained example. Tool/provider readiness is still
  checked when loading an example; unavailable dependencies are not silently replaced.

Verify source snapshot isolation, no execution on open, restart, and source-run deletion.

### AC14 — Compare immutable attempts

- Pin a completed run and compare another saved attempt of the same task/operation. Identify
  the two runs and show saved settings, changed values, compatible outputs, and labeled spend.
- Draft edits do not alter either comparison side. Pins survive reload; clearing a pin does
  not delete a run. Deletion of a pinned run handles its pin explicitly.
- Changed input files/frames are called out. This is manual review, with no invented quality
  score, automatic winner, or batch evaluation.

Verify two attempts, subsequent draft edits, task switching, pin reload, and differing inputs.

### AC15 — Image comparison is honest and operable

- Offer slider and side-by-side comparison for compatible image artifacts. Label the actual
  sides: Source/Result, Source/Mask overlay, Source/Location overlay, or Baseline/Attempt.
- Preserve aspect ratios and declared frames. Do not stretch unrelated images into apparent
  alignment or use a mask preview as evidence of an edited result.
- The divider supports pointer/touch and keyboard adjustment, with accessible names and values.
  Raw data remains available for non-image outputs; specialized charts stay tool-owned.

Verify matched/mismatched frame fixtures, endpoints, keyboard control, and narrow viewports.

### AC16 — One shared drawer

- Pipeline handoff, run details, browsing examples, and saving examples use the same right-side
  drawer. Its header identifies the content and selected run/tool where relevant.
- Opening supporting content does not append large panels to the main workspace. Close/Escape
  dismisses the drawer and returns focus to the trigger. Modal focus stays within the drawer.
- Long content scrolls inside it; narrow screens use available width; reduced motion is honored.
  Tool switching and loading another run cannot leave a stale drawer bound to the wrong run.

Verify each drawer use with keyboard, long content, focus restoration, and real browser layout.

## Take it into a pipeline

### AC17 — Copy a tested invocation

- Use in your pipeline exposes the selected run's recorded MCP call and supported CLI mapping,
  effective configuration where available, prerequisites, input roles, and versions.
- Editing the current form does not rewrite that run's copy/export. Untested draft snippets,
  if offered, are labeled separately. Never infer CLI flags from presentation labels.
- MCP export includes the recorded call plus declared startup/input-grants/protocol/prerequisite data;
  label it setup data for an MCP client, not a standalone program. Generated client programs are
  optional; if offered they must include initialization/dependencies and pass independent replay.
  CLI quoting is correct for its declared shell. Unsupported formats are identified without fake code.
- Environment-variable names may be shown; credential values never appear in exports.

Verify hostile/space-containing paths, immutable snapshots, and an independent offline replay.
M5.21 verifies resolved CLI/effective invocation under C2 within declared portability.

### AC18 — Download includes required inputs

- By default include supported invocation/setup, configuration, manifest, required images, and
  supported typed artifacts. Show included files and size; offer reference-only mode explicitly.
- A portable example uses resolved bundle-relative inputs and works outside Wafflery with
  the declared tools/providers available. It does not require Wafflery at runtime.
- If an artifact cannot be exported/imported or reconstructed, state the local workspace
  dependency. Do not label a manifest-only download as a portable runnable bundle.
- Verify fingerprints and omit credentials, unrelated files, tool binaries, and model installs.
  Explain absent assets. Nondeterministic providers are not promised identical output pixels.

M5.21 verifies copied native/image bundles with the old workspace unavailable under C2/R3. Nonportable inputs remain explicitly workspace-dependent.

### AC19 — Reports remain tool-owned

- Host the tool's declared report inside the shared inspection surface using an agreed readiness
  address and lifetime protocol. Do not scrape human terminal text to find an address.
- Surface report startup failure separately from run failure. Closing/reopening a drawer must
  not lose the run or leak report processes. Iframe policy permits the intended local host only.
- Report reconstruction/preservation follows a declared interface, not hardcoded file layouts.

Verify dynamic ports, slow startup, unsupported framing, failure, shutdown, and reopen. See C3.

## Engineering and release gates

### AC20 — Shared components stay shared

- React/TypeScript/Vite UI uses a small owned library with selected shadcn foundations.
  Task/operation picker, drawer, comparison viewer, and common fields share implementations.
- Storybook covers their actual states, including long labels/content, missing prerequisites,
  errors, keyboard behavior, and narrow widths. No unused catalogue of speculative components.
- Product checks use real browsers; component documentation alone is not accessibility proof.

### AC21 — One local distribution

- First-release support targets macOS. Linux is a possible low-priority follow-up; Windows is
  outside initial scope. Verify installation, host lifetime, file access, and packaging on macOS.
- SQLite runs inside the host, opens on startup, and closes on shutdown. Saved metadata survives
  host shutdown/relaunch; no separately running database service is required.
- Installed release serves its embedded UI on loopback and uses MCP stdio for tool execution.
  End users do not install a JS toolchain; source distributions support a Rust host build using
  prebuilt assets. UI regeneration and asset-freshness checks belong to development/CI.
- Storybook and mock scripts are not production runtime dependencies. No development CDN or
  internet resource is required to render the installed UI.
- Confirm macOS versions/architectures and browser support, dependency pins, build commands, source layout,
  and release mechanics in the engineering spec before the build plan is approved.

### AC22 — Boundaries and failure recovery are testable

- A fake conforming tool drives discovery, schema forms, progress, cancellation, runs, reports,
  snapshots, and exports offline. Its names differ from the real tools to detect special cases.
- Stage fake support against available contract versions. The first v0.1 milestone does not
  invent classification/readiness, typed bindings, or report/export fields. Add those after
  owner-reviewed declarations; interim neutral Actions and Storybook fixtures are not a claim
  that the full Playground/Operations release criteria have passed.
- Validate provider/tool results and artifact locators; constrain files to allowed roots. Serve
  local data only to the authorized local session; account for browser origins and WS access.
- Inline only verified allowlisted raster assets, with explicit Content-Type and nosniff.
  SVG/HTML/unsupported files download as attachments; malicious imports/tool output must not
  execute on the authenticated UI origin. Apply a restrictive UI CSP and isolated report origin.
- Failed description, missing executable, malformed output, dying child, missing assets, and
  host reconnect have explicit states. No blind retries that can duplicate paid work.
- Verification names the accepted tool/contract revision. Mock wiring and source inspection are
  not substituted for runtime integration evidence. Live paid checks need explicit scope/cost.

## Feature and release gates

Q19 scopes these gates to the work they govern. M0–M3 and V01–V19 are complete; M5.21 verifies the
v0.4 operations/invocation client against approved peers. The following are current dispositions,
not a blanket production release approval.

| Gate | Verified | Remaining boundary |
| --- | --- | --- |
| G1 | Owned images, evidence, declared native bytes, examples and invocation bundles; host/CLI replay after source loss. | Tool-owned reports still need usable run directories. Nonportable types remain workspace-dependent. |
| G2 | Owned child/report lifetimes, explicit cancellation/restart and session cap/budget/quote controls. | Minimal interruption reporting; no durable job recovery or cross-restart spending service. |
| G3 | Canonical H1 launch and compatible-host attachment. | Real installation channel, automatic installed-tool discovery and packaged entry journeys. |
| G4 | Explicit `serve --allow-dir` input grants replace roots; evidence reads remain separate; ownership-aware cleanup. | User-selected external stores remain separately owned. No automatic old-root registration migration. |
| G5 | Generic task/operation/utility discovery, port bindings and supported artifacts verified through independent and real peers. | Unadvertised capabilities/provider quality are not inferred. R2 remains optional. |
| G6 | Locked Rust/UI dependencies, embedded UI, reproducibility checks and hosted macOS CI. | M7 release mechanics, macOS minimum/architectures and final distribution acceptance. |

Chromium/WebKit coverage is not a substitute for native Safari manual evidence. Real paid-provider
quality/billing and evaluations are outside the offline M5 acceptance. V20 import feedback remains
an adjacent task; the host must not guess Squint's undeclared pixel limit or silently resize inputs.
