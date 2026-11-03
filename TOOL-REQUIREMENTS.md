# Tool and contract handoffs

Current status, 2026-11-03. The original requests below retain their IDs and historical audit.
The published contract is `=0.4.0`; the host is verified against approved Rascal `eed6397` and Squint
`6dd1e3a`. See [M5 acceptance](docs/reviews/2026-11-03-m5-operations.md).

| Request | Current disposition |
| --- | --- |
| C1 / S1 | Readiness/setup, list images, bound estimates and task discovery verified. |
| R3 | Declared operations/ports/artifact resources, independent testing and explicit chaining verified. |
| C2 | Effective invocation, declared native export/import and host/CLI bundles verified. Nonportable types remain workspace-dependent. |
| C3 | Server/document framing and lifetimes verified; tool-owned report reconstruction remains unsupported. |
| C4 | Association, explicit session restart, quotes/caps/budgets and preserved spend verified with controlled peers. |
| H1 | Canonical launcher and host attachment implemented; distribution/install channel remains open. |
| R1 / R2 | Advertised tasks remain tool-owned. Real provider/finder quality and optional prompt-led damage localization are not certified by offline host tests. |

The earlier packets are historical requests, not an instruction to rebuild settled wire shapes or
to edit sibling repositories. New cross-tool decisions go through the shared-contract coordinator.

## Owner packets

| Owner | Packet | Main work |
|---|---|---|
| table-manners + host coordination | [Shared interface](docs/handoffs/table-manners.md) | Generic readiness/intent/operation discovery, invocation/artifact portability, report lifecycle, spending sessions, launcher agreement |
| Rascal | [Rascal handoff](docs/handoffs/rascal.md) | Finish existing recipe-action integration; expose task intent and native operation/artifact bindings; align launcher; export supported dependencies |
| Squint | [Squint handoff](docs/handoffs/squint.md) | Remaining unconfigured bootstrap/settings/estimates mapping, reports, invocation export and H1 launcher; callable names resolved |

Each packet contains current source evidence, requested behavior, deliverables, offline
acceptance tests, and source links. They can be handed to the owning agents individually.

## Request IDs and release impact

| ID | Request | Acceptance coverage | Release impact |
|---|---|---|---|
| R1 | Rascal task intent/readiness and complete outcomes | AC04/06/08/09 | Required for automatic tasks; recipe actions merged in M8 |
| R2 | Prompt-led damage localization | AC06 | Optional new capability; unavailable until declared |
| R3 | Independent operations and native typed artifacts | AC03/05/11/13/17/18 | Required for deep Operations workspace |
| S1 | Squint callable checks, configuration and readiness | AC03/04/07/08/09/10 | Required for real Squint integration |
| C1 | Generic discovery, intent, readiness, estimates | AC02–08 | Required for tool-independent forms/readiness |
| C2 | Effective invocation/configuration and portable dependencies | AC05/11/13/17/18 | Required for reproducible handoff beyond recorded MCP calls |
| C3 | Report readiness, address, framing and lifetime | AC16/19 | Required for embedding tool-owned pages |
| C4 | Budget/session identity and recovery | AC08–10/22 | Required for dependable paid execution |
| H1 | Shared launcher/bootstrap agreement | AC01/02/10 | Required for all three entry journeys |

R1–R3, S1, and C1–C2 retain their earlier request IDs. C3/C4 separate existing broad shared
requests into reviewable units. H1 is a host/tool agreement; it need not become a core schema.

## Audit boundary

Rechecked 2026-10-31; see [source inventory](docs/integration-sources.md) for paths and observations.

- table-manners `8e4d0c3`: published v0.1.0, Squint fixtures present, v0.2 proposals untagged.
- Rascal `f0e45fb`: clean, M8 merged; `m8` targets `ebb773f`. Free no-config recipe actions exist.
- Squint `eef1f10`: clean local main, 15 commits ahead of local origin/main, no m7 tag/checkpoint.
  Serve and five use-case tools exist; estimates create runs. Testkit serve already supports a
  scripted provider before credential resolution, contrary to the external review's assumption.

No sibling test suites or paid calls were run for this audit. Historical review snapshots are
retained as dated evidence; they do not describe current implementation status.

## What the contract already provides

Descriptions and parameter schemas, one MCP tool per described action, stdio server conventions,
progress/cancellation, run resources, result status/spend/cache, image roles and identities,
location vocabulary, cost flags, an estimate obligation, and render command templates.

The host can capture actual MCP calls today. That does not automatically provide CLI mappings,
effective defaults, portable native artifacts, a full runnable client example, or a report's
ready URL/lifetime. Existing Rascal compatibility export and Squint report export are not the
same thing as exporting a reproducible invocation bundle.

## Host-owned work

Wafflery owns the local host, process discovery/reuse, browser API/session boundaries,
managed input snapshots/deduplication, run/example/baseline records, explicit cleanup, the shared
React/Storybook UI, drawer/image comparison, copying recorded calls, and assembling downloads.
Tools own execution, providers/secrets, configuration semantics, authoritative spend limits,
artifact meaning, and detailed reports. Shared declarations bridge those responsibilities.

Snapshotting inputs cannot solve unavailable native artifact import/export, and storing a run
reference cannot guarantee a report survives pruning of tool-owned assets. Output/report ownership
and the other [feature gates](ACCEPTANCE.md#feature-and-release-gates) still
need an engineering decision before the plan is locked.

The owner approved staging those dependencies: [PLAN.md](PLAN.md)'s foundation can proceed
independently. Gate each native/report/paid/real-tool integration on its required agreement,
not the whole host and UI effort. Full first-release acceptance remains unchanged.

[ENGINEERING.md](ENGINEERING.md) now proposes local ownership and lifecycle rules. It keeps
tool startup/workspace declarations (H1), native preservation (C2/R3), and durable report input
support (C3) explicit. Review these concrete boundaries with the owners; the draft does not
pretend those additions already exist in v0.1.

## Historical coordination order

Urgent deliverables are ready: [H1 launcher proposal](docs/handoffs/H1-launcher-proposal.md),
[C4 v0.2 proposal](docs/handoffs/C4-v0.2-proposal.md), and the
[table-manners proposal patch](docs/handoffs/table-manners-v0.2-proposals.patch).
H1 is owned here and must be agreed now; C4 should be reviewed before v0.2 is tagged.

1. Give the shared owner C1 and the concrete R3 operation/artifact examples together. Request a
   versioned proposal and paired fixtures, not ad hoc Wafflery-only fields.
2. Rascal M8 is merged; request a scheduled R3 delivery milestone and remaining R1 metadata.
   Squint's naming issue is resolved; remaining S1 bootstrap/metadata and M7 closure stay explicit.
3. Resolve C3 against both real viewers. Develop the fake tool/viewer in Wafflery first.
4. Resolve C2 against a saved native Mask and a Squint check; verify portable replay offline.
5. Agree C4 and H1 with both tool owners and the host before paid integration/release packaging.

Return a revision, schemas/fixtures, callable examples, test evidence, and explicit remaining
exceptions for each handoff. No paid external calls are needed for this acceptance work.
