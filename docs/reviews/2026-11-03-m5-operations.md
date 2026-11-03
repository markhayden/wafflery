# M5 operations and invocation acceptance

Status: **M5 complete, 2026-11-03.** Implementation, real-peer acceptance, local checks and independent
review are complete. The full hosted matrix passed on `e840e91`,
[run 41646196843](https://github.com/markhayden/wafflery/actions/runs/41646196843), including the
closeout corrections below. V20 image-import feedback remains the adjacent follow-up. This is not
M7 distribution/release approval.

## Contract and peers

The exact crates.io pin is `table-manners = "=0.4.0"`, with Cargo.lock checksum
`033f2506fdaefc7be15f56eaacfe41d844b44b3a626779b43e657e8caaa0a0b0`. The embedded schema tests check
this version and the published operation/control schemas. MCP negotiation remains `2025-11-25`.
No sibling contract path pin, compatibility shim or MCP-root fallback is present.

Approved isolated source builds: Rascal `eed6397844c5346caec63670cb76ed920b83d9a2` (reports 0.1.1)
and Squint `6dd1e3a99c60c146ecb6fde7f8839fc1a9727e51` (reports 0.1.0). Their locked testkit builds
resolve published contract 0.4.0. [Integration evidence](../integration-sources.md) records the actual
native/task/CLI/browser probes. No sibling working files, provider credentials or paid services were
used. Scripted/synthetic charges verify accounting behavior, not real-provider quality or billing.

## Accepted behavior

| Area | Evidence |
| --- | --- |
| C1 / discovery and readiness | Exact live-version gates; action readiness/setup, bound estimates/caps, indexed refusals and list slots; independent fake plus real setup probes. |
| R3 / independent operations | Generic kind/group/port/type discovery; typed image/location/opaque native inputs, ordered lists, saved references and artifact resources. Real Line → Rasterize → Feather, including parameter edits without hidden upstream runs. |
| C2 / preservation and invocation | Resolved effective invocation on saved outcomes, separate native preservation state, owned native exports/imports, independent example ownership and portable input bundles. Real replay works after original inputs/store/run directories are removed. |
| Chains | Explicit ordered selection with source-lineage agreement, shared manifest-edge resolution, fresh public output port/item binding, ordinary per-step Run and fresh paid estimate, explicit Stop/resume. Nondeterminism is covered by the independent peer, not inferred from equal deterministic hashes. |
| C3 / reports | Both server/document viewers use the serving executable, exact framing origin and owned lifetimes. Real reports pass strict console and rendered-style assertions in Chromium/WebKit. Tool-owned report source directories remain a dependency. |
| C4 / spending | Estimates are controls, no silent session restart/refill, per-step caps/quotes, cancellation ownership and retained completed evidence/spend. Uncertain paid acknowledgements retain the original request ID. |
| File boundary | `serve --allow-dir` grants only execution staging plus explicitly registered inputs; effective configured access is displayed. Evidence-only directories never become caller input grants. |

No claim is made for portable Plan/Series/BandProfile beyond the tool's declaration, arbitrary artifact
store enumeration, tool-private decoding, automatic reconstruction, batch evaluation, agent loops,
or a background cross-tool workflow engine. Native previews remain visual aids, not native inputs.

## Interface comparison

Capture uses the real approved offline Rascal and its small synthetic input image at 1440 × 1000.
The original mock is the checked-in focused-picker study in `work/mockups.html`/`activity-hierarchy`.
No machine paths, tokens, credentials or user images are shown. The mock's sample photo is retained
from the original study. New screenshot regressions use synthetic Storybook regions and the existing
shared visual policy; no tolerance was relaxed.

| Area | Original | Implemented / decision |
| --- | --- | --- |
| Tool and activity hierarchy | [Focused operations](m5-images/mock-operations.png) | [Chromium operations](m5-images/operations-chromium.png), [WebKit operations](m5-images/operations-webkit.png). Tool stays in the header, then workspace kind and operation share matched focused pickers. |
| Long operation purpose | The mock trigger uses a short input → output signature. | Capture found the full Feather purpose expanding its trigger. The correction uses declared port-type summaries (and image/location summaries for tasks) in the closed control; full purpose remains searchable in its chooser and present in the form. Long task/operation purpose stories check matching heights at 390/1440 in both engines. |
| Typed inputs and result | Mock Feather demonstrates a saved mask and independent parameters. | Actual native references, resource compatibility, preserved bytes and precise sigma inputs remain visible. Optional tool fields and preservation status take more room than the illustrative mock. |
| Drawer and replay | [Journey study](../../work/journey-ui.html) and [drawer study](../../work/drawer-ui.html) | Shared drawer contains original invocation, files and ordered per-step chain progress. Explicit import, Run/cap review and settling/Stop/resume states replace optimistic mock buttons. |
| Responsiveness and semantics | Mock narrow single-column layout | Existing desktop scrolling/fixed footers, narrow page flow, 44px controls, keyboard focus and modal semantics remain. Region screenshots cover hierarchy, bindings, bundles and replay states. |
| Native output | Mock line overlay / mask preview | Public location overlays and verified raster previews coexist with opaque native identities. A rendered preview is never substituted for an unavailable native artifact. |

The shared drawer and ActivityPicker stories cover loading, empty, setup/refusal, changed identity,
stopped/settling and completed states. Full tool report HTML remains isolated and unmodified; it is
not restyled to resemble the mock. Native Safari manual coverage is separate: M3's owner-reported
Safari check does not certify new M5 operations or reports.

## Bounds, storage and rollback

- Library schema 8 owns bundles separately from attempts/examples. Original bytes, native blobs and
  bundle input owners survive source cleanup; examples retain their own required bindings.
- Archives are canonical uncompressed ustar: maximum 256 MiB whole archive, 8 MiB manifest, 64 MiB
  other member, 64 members including the manifest. Reads validate headers, names/order, JSON,
  claims, fingerprints and padding before tool contact. Oversized/unsupported imports refuse.
- Chains contain at most 16 explicitly selected steps. Two upload reviews and two guided chain
  reviews may be open. Each manifest resolution and fresh-output normalization has a bounded work
  budget; all fresh producers share the normalization budget.
- Ambiguous public occurrences or conflicting recorded lineage refuse runnable chain export;
  individual-step export remains available. No private `from_step` field is added.
- Missing outputs, unresolved native resources and preservation failures block affected downstream
  inputs, while completed/partial evidence and actual spend remain saved. Resume never silently
  substitutes archived native copies or repeats an upstream producer.
- Old `roots` registrations require explicit backup/re-registration using the README procedure.
  `allow_dirs` and `evidence_dirs` are distinct. Native Rascal export preservation requires an
  explicitly registered read-only export area; there is no host path guess.
- Stop/drain active owners before reverting replay/import entry points. Preserve parsing of new
  optional context metadata and schema-8 ownership for retained records. Older binaries may not
  understand new records; use a separate library or restore a matching backup for downgrade.

Remaining release work includes automatic tool discovery, a real installer/update/distribution
channel, supported macOS minimum/architectures and final packaged entry journeys. Reports continue
to depend on usable tool-owned run sources. Bundles do not expire; dedicated bundle deletion is not
yet provided. No real library migration or public release was performed in this milestone.

## Verification record

M5.20: full locked Rust workspace tests, formatting, all-target clippy, UI typecheck and 94 unit tests;
134 targeted Chromium/WebKit cases plus a final 58-case unchanged-baseline run. Independent review
confirmed per-step paid approval and Stop/Run serialization; optional decisions are recorded in PLAN.

M5.21: full Rust/lint/type/unit checks, five opt-in real-peer Rust checks and six real browser journeys
passed. Review fixed unsafe environment-folder cleanup and a previous-result browser assertion;
follow-up review found no blocker. CLI subprocesses have bounded waits and kill-on-drop ownership.

M5.22: the long-purpose test failed before the picker fix, then 48 targeted Chromium/WebKit cases
passed, including paid Run/outcome regressions and workspace fidelity. Final full locked Rust tests,
formatting, all-target clippy, UI typecheck, all 96 unit tests, asset-manifest tests, two clean
byte-identical UI builds, published-schema checks and release build passed. Independent review
approved the UI/doc corrections. Hosted CI found a separate combined setup/body deadline issue in
the new replay spec; [analysis and correction](2026-11-03-chain-replay-deadline.md) include a controlled
old-fails/new-passes reproduction, twenty final WebKit repetitions and 26 unchanged-baseline replay
checks in both engines. The correction changes no product lifetime or retry policy. Hosted CI runs
the entire matrix; local verification follows the coordinator's lean policy rather than rerunning
the complete browser suite.

## Bundle-source fixture correction

[Hosted run 41639928503](https://github.com/markhayden/wafflery/actions/runs/41639928503)
correctly refused a bundle assembled from an early saved snapshot. Native bytes were already
preserved while contract evidence capture remained pending. Waiting later without replacing that
snapshot did not make it usable. The shared test helper now waits for run settlement before returning
a fresh snapshot with native preservation complete. New assertions reproduced the pending evidence
locally before the fix. All live bundle import/replay source callers share that helper; static fixtures
and real-peer probes were checked separately. Product refusal and worker lifetimes remain unchanged.

## Closed chooser descriptions

[Hosted run 41641952288](https://github.com/markhayden/wafflery/actions/runs/41641952288)
passed Rust, then found the refreshed form purpose twice: once in the form and once in a closed
chooser's retained hidden choices. Choices now mount only while the chooser is open. This keeps
full descriptive choice text when selecting, the current purpose in the input form and the compact
closed trigger. A regression failed in both engines before the correction and verifies selection
removes hidden choices. The same run found two readiness cases using an obsolete combined
choice label; those now select the stable title inside the chooser. Native focus restoration and
open/close behavior remain covered; no timing or retry policy changed.
