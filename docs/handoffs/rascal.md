# Rascal handoff: tasks, operations, and reproducible runs

Status: ready for owner review. Keep implementation in Rascal; shared additions go through
table-manners. Wafflehouse remains a generic client. No sibling files changed.

## Current evidence

Rechecked 2026-10-31: HEAD `f0e45fb`, clean. M8 is merged and tag `m8` points to `ebb773f`,
an ancestor. Tasks 8.6/8.7 are implemented, not open. See the [source inventory](../integration-sources.md#rascal).
This inspection did not rerun Rascal's checks; the milestone's recorded evidence remains tool-owned.

Recipe actions (`seam`, `remove`, `remove_prompt`, `repair`, `repair_area`, `protect`, `mask_rect`)
and native op tools exist. Without config, the description advertises no paid filler and empty
credential requirements. `mask_rect`, optionally `seam` without filler, supplies an opt-in real
offline M2 smoke through explicit test registration after Wafflehouse's no-paid guard is tested.

## R1 — Make complete-task intent discoverable

**Request:** finish/verify the existing recipe-action path and close its presentation/readiness
gaps. Wafflehouse must offer complete outcomes with configured defaults, not construct recipes.

- Wrap-seam repair takes a panorama and filler; its recipe already cuts the wrap edge into a
  working canvas and defines the center line. It does not detect arbitrary seams.
- Prompt removal uses the finder and filler. Mask creation uses the finder without a filler.
- A generic client must distinguish those intents and their prerequisites, including when the
  same underlying action changes output depending on `filler`. A missing filler cannot silently
  satisfy a UI promise of an edited image. Agree metadata with C1; separate wire actions are not
  prescribed if a supported configuration/intent declaration solves it.
- Resolve defaults, supported overrides, finder versus fill instructions, and effective settings
  inside Rascal. Describe choices; don't require the host to parse private config or step names.
- Verify the completed cost/cache/enforcement integration and retained salvageable masks on fill failure; do not duplicate merged M8 work.

**Accept:** an independent client discovers and runs seam repair with a fake filler, prompt
removal with controlled finder/filler, and mask-only creation with no filler call. Validate output
roles and preservation outside allowed support. Missing dependencies, provider failure, and
cancellation expose terminal state/spend/partial output without fabricated success. AC04/06/08/09.

**Deliver:** accepted description/readiness fixtures, the task-to-action/configuration mapping,
offline execution tests, and explicit remaining integration gaps after merged M8. Recipe-level
MCP action creation and spend/cache infrastructure already exist.

## R3 — First-class independent operations and typed artifacts

**Request:** provide a shared way for the host to discover/bind/run/inspect native operations.
Accepted ADR 0020 leaves op envelopes/mapping unchanged in v0.1. No R3 milestone is currently
scheduled; request a concrete next-milestone commitment rather than assuming v0.2 alone delivers it. That is not yet a generic contract
for the Operations workspace, despite native CLI/MCP calls already being available.

Agree with C1/C2 how to expose operation purpose/category, typed inputs/outputs, parameters, and
saved artifact references. Required types include Image, Location, Mask, BandProfile, Series,
and Plan. Preserve frame/identity compatibility. Scope metadata to actual capabilities rather
than embedding a second Rascal implementation in the playground.

**Accept:**

1. Run Line independently on an image; show its Location geometry and data; no finder/filler call.
2. Bind that Location to Rasterize through declared types and references.
3. Bind a saved native Mask to Feather, change sigma/support, and run only Feather. Inspect both
   artifacts, and reopen the binding after host restart.
4. Wrong types/frames and unavailable artifacts fail clearly. Opening a recorded step is supported
   only when its necessary inputs are available; never rerun upstream steps silently.
5. Export/import that saved Mask or supply a supported reconstruction recipe; otherwise report
   the workspace dependency. The shared rendered-mask location is not a native Mask replacement.

**Deliver:** one complete typed-operation declaration/binding example plus positive/negative
fixtures and independent-client tests. Include persistence and relocation behavior for C2.
AC03/05/11/13/17/18. This is a first-release blocker for generic deep operation access.

## R2 — Optional new capability: prompt-led damage localization

Scratch repair currently takes a stroke; stain/area repair takes drawn geometry. A prompt such
as “repair the scratch” needs an actual localization design. Define supported defect classes,
no-match/ambiguous behavior, and conversion to valid repair inputs. Do not assume object
segmentation yields the required stroke. Demonstrate each supported class with representative
offline fixtures and preserve manual-override provenance.

This is new tool capability, not simple UI exposure. Keep the proposed mock task unavailable
until supported; it need not block R1/R3. AC06.

## Shared dependencies and launcher correction

- C2: provide CLI invocation mapping and resolved settings for the selected run. Existing
  `export --compat` projects result envelopes, drops native artifact bookkeeping, and is not a
  runnable portable example exporter.
- C3: `dash` already prints a bare loopback URL; adopt the agreed shared readiness/lifetime and
  framing contract. Decide report asset ownership/relocation.
- C4: distinguish cancellation requested from completed; current docs say an in-flight filler
  may finish under its timeout. The UI must retain accurate spend and completed output.
- H1: implemented `playground` currently finds `PLAYGROUND`/`playground` and offers no install.
  Review the [canonical launcher proposal](H1-launcher-proposal.md), then land a corrective change
  after M8. Wafflehouse proposes `WAFFLEHOUSE_BIN`/`wafflehouse` and `~/.local/bin/wafflehouse`.
- C4: respond to the [v0.2 proposal](C4-v0.2-proposal.md) with structured run association and
  observable session boundaries; human progress-message formats will not be parsed by the host.

## Verification and response

Use Rascal's existing independent MCP client, fake pack/filler fixtures, schema validation,
and established checks. Do not make live paid calls for these acceptance cases. Return the
implemented revision, description/fixtures, supported invocation examples, test results, and any
remaining unsupported states. Product criteria live in [ACCEPTANCE.md](../../ACCEPTANCE.md).

## Sources checked

- [Description](../../../rascal/crates/rascal-mask/src/describe.rs),
  [action execution](../../../rascal/crates/rascal-mask/src/actions.rs),
  [contract notes](../../../rascal/docs/contract.md), [M8 plan](../../../rascal/PLAN.md).
- [Seam recipe](../../../rascal/crates/rascal-mask/recipes/seam.toml),
  [recipe documentation](../../../rascal/docs/recipes.md), [fillers](../../../rascal/docs/fillers.md).
- [Existing compatibility export](../../../rascal/docs/export.md).
- [Shared requests C1–C4/H1](table-manners.md).
