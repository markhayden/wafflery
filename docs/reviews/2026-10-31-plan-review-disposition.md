# Foundation plan review: disposition

Date: 2026-10-31. Response to the owner's supplied external review of M0–M3.
The [source inventory](../integration-sources.md) records inspected revisions and limits.
This is planning work; no application implementation or sibling test run is claimed.

| Finding | Disposition |
|---|---|
| H1 is a host-owned dependency now | Drafted the [launcher agreement](../handoffs/H1-launcher-proposal.md) now: binary, override, lookup/install location, argv, exits and private host attachment. F05a/b verify host behavior; actual launcher/installer integration remains M4. Rascal's implemented launcher needs correction; Squint 7.9 awaits agreement. |
| Use a real offline target | F13r adds an ignored, opt-in no-config Rascal smoke after the dispatch guard. Same generic descriptor/client as the fake, isolated directories, no credentials. It is outside required checks. |
| No-paid guard lacked ownership | F11a fails closed unless current validated cost.may_spend is false, including direct API requests. Tests count zero execution calls for prohibited or stale declarations. |
| Deletion checks precede deletion | F14/F15 check storage and references; F17 owns actual deletion, baseline clearing and example-survival checks. |
| Contract v0.2 drift | Initial exact execution allowlist is 0.1.0. Published crate/schema fixtures, deliberate tagged upgrade, negative unknown-version fake, array-capable internal bindings with version-specific cardinality. Roots deprecation is an explicit risk; MCP remains pinned. |
| C4 cannot remain a passive wait | Drafted [C4 semantics](../handoffs/C4-v0.2-proposal.md) and a standalone [PROPOSALS.md patch](../handoffs/table-manners-v0.2-proposals.patch). Structured run association, server-session identity and budget observation require schema-owner agreement. Unknown outcome after early cancellation is a normal v0.1 state. |
| Estimates can create runs | Recorded Squint's finished() behavior in engineering and C1 handoffs. Control records need not become user history. |
| Core operations lack a supplier | R3 is explicitly unscheduled, required for full Operations coverage, and an owner scheduling decision. Recommended next Rascal milestone; not recorded as agreed. C2/C3 need delivery checkpoints too. Foundation Actions does not satisfy full release coverage. |

Task slices now name the previously implicit Vite, test-script, browser and packaging work.
Tests/stories count toward authored-file budgets; larger tasks are split, including shutdown
backend and UI. Generated assets and lockfiles are reviewed separately. UI, library and fake
tracks have independent dependencies. F04 proves two clean byte-identical UI builds, warns on
stale assets in debug, and fails in release/strict CI; missing assets always fail. F16 owns
archive writer dependencies. Fixture validation uses the published crate and URN retriever.

Two review observations needed qualification:

- Squint already has testkit-only serve --provider-script. ScriptedProvider is selected before
  credential resolution, with existing server tests. No new provider request was added.
- This shell reports Node 22.13.1 via Volta; the review reports 24.18.0. Preserve both environment
  observations and record the actual executable/version in M0. Browser caches exist, but actual
  compatible browser launch still needs verification.

Active source claims and owner packets were refreshed. Dated review snapshots retain their
original evidence with a superseding notice. Squint links use repository identity and a source
inventory instead of depending on its temporary checkout folder name.

Validation: the proposal patch passes git apply --check in table-manners. It has not been
applied, sent, or accepted; sibling repositories were not edited. Document links, task dependencies
and retained acceptance/decision IDs are checked separately from future application tests.
Implementation approval of the revised M0–M3 plan remains pending.
