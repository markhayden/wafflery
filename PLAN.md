# Wafflery foundation implementation plan

Repository update (2026-11-01): the canonical checkout is `wafflery`; the coordinator has
published main and manages subsequent pushes. The [CI gate](docs/plans/ci.md) runs this plan's
full matrix on `macos-15`; its [checkpoint](docs/reviews/2026-11-01-ci.md) separates local checks
from the first hosted run. Local merges remain subject to review; no automatic pushes or tags.

Status: M0–M3 approved by the owner on 2026-10-31; implementation started.
Current implementation evidence: [M0 checkpoint](docs/reviews/2026-10-31-m0-checkpoint.md) and
[M1 checkpoint](docs/reviews/2026-10-31-m1-checkpoint.md), and
[M2 checkpoint](docs/reviews/2026-10-31-m2-checkpoint.md), and
[M3 checkpoint](docs/reviews/2026-10-31-m3-checkpoint.md). M0–M3 are complete: automated and
packaged-source checks pass, and the owner explicitly confirmed the native Safari checklist.
M4's explicit real-Rascal entry slice is merged on main, including registration and the real
launcher/browser journey. See its [checkpoint](docs/reviews/2026-10-31-m4-entry-checkpoint.md).
The Squint offline entry and published v0.2 integration are also merged, as is the shared
Tool setup drawer with live session spending. M4 remains incomplete: automatic installed-tool
discovery, production installer/registration, unconfigured tool readiness and provider setup,
and the complete first-install journeys from all three entry points still need delivery.
The [current review disposition](docs/reviews/2026-10-31-plan-review-disposition.md) records
changes and validation limits; it is not implementation verification.
The owner agreed to proceed with foundation planning independently of later tool interfaces.
The owner authorized continuing into real-tool integration on 2026-10-31; the concrete next
slices are in [M4: real tool entry](docs/plans/m4-real-tool-entry.md). Paid runs are also authorized
once estimates/caps/session handling are implemented; the current dispatch guard remains offline-only.
Publishing and production installation are not authorized by this continuation.

Current integration update (2026-11-01): published v0.2.0 and both tools are adopted on main.
See [the v0.2 integration plan](docs/plans/table-manners-v02.md) and [verified checkpoint](docs/reviews/2026-11-01-table-manners-v02.md). The initial v0.1 fixture/version
requirements below describe the completed foundation checkpoint, not the current live allowlist.
The [v0.3 client task list](#v03-client--proposed-task-list) below is approved. V01–V03 are
merged locally; the live client requires exactly v0.3 and still refuses paid dispatch until
its estimate/cap guards are implemented. See [execution checkpoints](docs/plans/v03-execution.md).

## Outcome

Deliver one locally running development build that can open a generic fake tool, accept an
image and parameters, execute a real offline MCP call, preserve the result, reopen history,
compare attempts, save/open an example, and copy the recorded call/setup data. It uses the
reviewed picker, drawer and image-comparison components, with Storybook alongside them.

The fake is clearly labeled. This milestone does not pretend to repair images, evaluate quality,
export a fully runnable native-artifact bundle, or integrate unimplemented tool protocols.

Design inputs: [SPEC](SPEC.md), [acceptance](ACCEPTANCE.md), [storage/process design](ENGINEERING.md),
[foundation runtime](docs/design/foundation-runtime.md), and [review corrections](docs/reviews/2026-10-30-spec-review-disposition.md).

## Dependency policy and coordination already brought forward

M0–M3 did not wait for all C1–C4/H1 features. Foundation planning brought forward the
[H1 launcher](docs/handoffs/H1-launcher-proposal.md) and
[C4 v0.2 semantics](docs/handoffs/C4-v0.2-proposal.md), with a historical
[table-manners proposal patch](docs/handoffs/table-manners-v0.2-proposals.patch).
Published v0.2 now supplies C4; the coordinator owns subsequent shared-contract decisions.
Historical drafts alone are not accepted wire interfaces.
F05 verifies H1's host semantics; M4 integrates the actual tool launchers/installer.

Use published `table-manners = "=0.1.0"` initially, with an exact live execution allowlist
`{"0.1.0"}`. No semver-wide acceptance. F09a tests rejection of an advertised unapproved `0.2.0`.
When v0.2 is tagged, adopt its crate/schemas/fixtures in an explicit dependency commit before
integration: add a valid 0.2 fake scenario and switch the supported set deliberately. Do not
maintain unnecessary unreleased compatibility shims. Old saved evidence remains readable;
re-execution requires a supported current declaration. A proposal is never accepted just because
a fake emits it. MCP protocol stays 2025-11-25 independently of the table-manners version.

Foundation execution permits registered offline fakes and an explicitly opted-in no-config
Rascal smoke descriptor. The host requires validated `cost.may_spend == false` at dispatch and
refuses true, missing, malformed or stale cost declarations. No provider credentials or live
network providers are needed. Never weaken this guard to run a scripted Squint action that
still advertises may_spend; Squint's own testkit already covers scripted checks.

Tasks below are atomic slices, with tests/stories counted as authored files. A task's listed
file budget is about five; generated output and lockfiles are separately reviewed. Each task
gets its own commit named for its title, with `chore:`, `feat:`, `test:` or `docs:` as appropriate.
No unnamed follow-up commits are required. Verification runs in the same task as the behavior.

Independent tracks after F01b: UI/tooling (F02–F04), library (F07), and fixtures/fake (F09).
F07 does not depend on browser auth. F09 does not depend on UI. They can be implemented in any
order respecting the explicit dependencies, without requiring parallel agents. Checkpoints
join these tracks rather than forcing a dependency that does not exist.

## M0 — Reproducible host and component workspace

### F01a — Record the reviewed project baseline

- **Accept:** Initialize Git only if still absent; preserve docs/mocks, ignore secrets/live data/build caches, and explicitly retain ui/prebuilt. No remote or push.
- **Files:** `.gitignore`; existing documents/mocks as the baseline snapshot.
- **Verify:** Inspect staged paths and secrets; confirm initial history and worktree state.
- **Depends:** none. **Commit:** `record the reviewed project baseline` with the appropriate conventional prefix.

### F01b — Establish the Rust package

- **Accept:** Harmless version/help executable and testable library compile under Rust 1.99.1; exact consumed dependencies and lock recorded.
- **Files:** `Cargo.toml`, `rust-toolchain.toml`, `src/main.rs`, `src/lib.rs`; Cargo.lock generated.
- **Verify:** `cargo build --locked`; `cargo fmt --all -- --check`.
- **Depends:** F01a. **Commit:** `establish the Rust package` with the appropriate conventional prefix.

### F02a — Lock the minimal React workspace

- **Accept:** React/Vite/TypeScript graph resolves without ignored peer errors; record actual Node path/version used. No live API yet.
- **Files:** `ui/package.json`, `ui/tsconfig.json`, `ui/index.html`, `ui/src/main.tsx`; bun.lock generated.
- **Verify:** Frozen install, advisory/peer check, `bun run --cwd ui typecheck`, `bun run --cwd ui build`.
- **Depends:** F01b. **Commit:** `lock the minimal React workspace` with the appropriate conventional prefix.

### F02b — Define UI tokens and Vite development settings

- **Accept:** Owned tokens and strict loopback dev configuration load; explicit host address is used for API development, not a permissive CORS wildcard.
- **Files:** `ui/vite.config.ts`, `ui/src/tokens.css`, `ui/src/main.tsx`, `ui/src/App.tsx`.
- **Verify:** UI build/typecheck; inspect no remote fonts/assets and usable empty shell.
- **Depends:** F02a. **Commit:** `define UI tokens and Vite development settings` with the appropriate conventional prefix.

### F03a — Create Storybook with the first real component

- **Accept:** Storybook shares production tokens; one accessible field/button component has meaningful stories, no speculative catalogue.
- **Files:** `ui/.storybook/main.ts`, `ui/.storybook/preview.ts`, component and story, `ui/package.json`.
- **Verify:** `bun run --cwd ui build:storybook`; typecheck; real-browser keyboard smoke.
- **Depends:** F02b. **Commit:** `create Storybook with the first real component` with the appropriate conventional prefix.

### F03b — Own unit and browser test entry points

- **Accept:** Create test:unit and test:browser scripts, Playwright Chromium/WebKit setup and fixtures. Cached browsers are checked for version compatibility and actual launch.
- **Files:** `ui/package.json`, `ui/playwright.config.ts`, `ui/tests/browser/fixtures.ts`, browser smoke, pure unit smoke.
- **Verify:** Run both scripts; record actual Node/Bun/browser versions; report blocked launch rather than substitute DOM checks.
- **Depends:** F03a. **Commit:** `own unit and browser test entry points` with the appropriate conventional prefix.

### F04a — Generate deterministic prebuilt UI

- **Accept:** Build/manifest scripts publish relative-path assets and input/output digests. Two clean builds with identical sources/toolchain have byte-identical outputs; no timestamps.
- **Files:** `scripts/build-ui.ts`, `scripts/check-ui.ts`, `scripts/ui-manifest.ts`, script tests; generated assets.
- **Verify:** Two clean temporary builds and hash comparison; changed-input/changed-output negative checks.
- **Depends:** F02b. **Commit:** `generate deterministic prebuilt UI` with the appropriate conventional prefix.

### F04b — Embed UI with a usable development loop

- **Accept:** Cargo embeds existing assets without Node/Bun. Missing assets always fail. Stale assets warn in debug but fail in release or WAFFLERY_VERIFY_UI=1; CI always checks freshness.
- **Files:** `build.rs`, `src/host/assets.rs`, module wiring, Cargo config, manifest/build tests.
- **Verify:** Rust build with JS absent from PATH; debug-stale warning, release-stale failure, strict-debug failure; fresh release build.
- **Depends:** F04a/F01b. **Commit:** `embed UI with a usable development loop` with the appropriate conventional prefix.

**Checkpoint M0:** Both builds, test entry points and Storybook work in the actual environment; dependency locks and repeatable asset generation are verified. Cached browsers lower download risk but do not waive execution checks.

## M1 — One host and a persistent local image library

### F05a — Serve one foreground host and verify H1 boundaries

- **Accept:** Foreground host serves UI on loopback with lifetime lock/private readiness. H1 draft explicitly names binary, lookup, argv, exits and host-only attachment; test doubles exercise that boundary.
- **Files:** `src/host/mod.rs`, `src/host/control.rs`, `src/main.rs`, `tests/host_lifecycle.rs`, H1 proposal.
- **Verify:** `cargo test --locked --test host_lifecycle`; simultaneous ownership and stale readiness cases. Proposal approval remains distinct from tests.
- **Depends:** F04b. **Commit:** `serve one foreground host and verify H1 boundaries` with the appropriate conventional prefix.

### F05b — Detach and attach the Wafflery launcher

- **Accept:** Default launcher starts/attaches, waits for readiness then opens the browser. Two launches converge; terminal/tab close leaves host alive; idle Quit stops it.
- **Files:** `src/host/launch.rs`, control/main wiring, lifecycle tests, README.
- **Verify:** MacOS concurrent launch, terminal exit, invalid override, incompatible host and explicit idle quit; no real installer.
- **Depends:** F05a. **Commit:** `detach and attach the Wafflery launcher` with the appropriate conventional prefix.

### F06a — Authenticate private browser APIs

- **Accept:** Single-use bootstrap, bearer auth, strict origin/Host checks and no cookie authority. Logs exclude secrets.
- **Files:** `src/host/auth.rs`, `src/host/api.rs`, host wiring, `tests/browser_boundary.rs`.
- **Verify:** API negative tests for expired/reused token, foreign origin, absent auth and header mismatch.
- **Depends:** F05a. **Commit:** `authenticate private browser APIs` with the appropriate conventional prefix.

### F06b — Connect authenticated browser sessions

- **Accept:** Exchange fragment once, remove it from history, restore per-tab session on reload; API client sends no ambient credentials to report origins.
- **Files:** `ui/src/api/client.ts`, bootstrap module, App wiring, browser session test.
- **Verify:** Real-browser bootstrap/reload; inspect sanitized URL, request headers and authentication failure UI.
- **Depends:** F06a/F03b. **Commit:** `connect authenticated browser sessions` with the appropriate conventional prefix.

### F07a — Open the local database worker

- **Accept:** Dedicated SQLite worker, initial migration and schema version open/reopen without a server dependency; migration failure preserves data.
- **Files:** `src/library/db.rs`, migration SQL, `src/library/mod.rs`, lib wiring, `tests/library_storage.rs`.
- **Verify:** Storage tests for effective PRAGMAs, reopen, write failure, newer schema and bounded worker shutdown.
- **Depends:** F01b. **Commit:** `open the local database worker` with the appropriate conventional prefix.

### F07b — Publish immutable deduplicated images

- **Accept:** Copy/hash/validate before reference commit; identical bytes share storage; originals never become mutable aliases.
- **Files:** `src/library/assets.rs`, asset migration, library wiring, storage tests, small owned image fixtures.
- **Verify:** Capture/publication fault injection, digest/dimensions, duplicate bytes, unstable original, full-disk/write errors.
- **Depends:** F07a. **Commit:** `publish immutable deduplicated images` with the appropriate conventional prefix.

### F08a — Expose safe authenticated asset transfer

- **Accept:** Stream bounded uploads and serve verified raster data by opaque ID. SVG/HTML/unsupported active content cannot execute on the UI origin.
- **Files:** asset API module, route wiring, `tests/asset_api.rs`, malicious file fixtures.
- **Verify:** Auth, path escape, size/decode limits, content types/nosniff and attachment tests.
- **Depends:** F06a/F07b. **Commit:** `expose safe authenticated asset transfer` with the appropriate conventional prefix.

### F08b — Build the reusable multi-image-capable input

- **Accept:** ImageInput stores ordered asset arrays and explicit min/max cardinality; v0.1 single slots still enforce exactly one supplied image. Upload/reorder/remove/errors are generic.
- **Files:** `ui/src/components/ImageInput.tsx`, story, binding model, unit test, browser test.
- **Verify:** Keyboard/narrow/long-list states, byte-identical files, Blob URL cleanup; internal arrays do not invent many:true on v0.1.
- **Depends:** F08a/F06b/F03b. **Commit:** `build the reusable multi-image-capable input` with the appropriate conventional prefix.

**Checkpoint M1:** Launch → authenticate → import image → restart → retrieve retained image succeeds. Library work joins auth only at the API boundary; no provider calls or real-tool installation.

## M2 — Execute offline MCP actions and check reality

### F09a — Validate contract fixtures and exact version policy

- **Accept:** Use published table-manners =0.1.0 dev-dependency and embedded-SCHEMAS retriever pattern. Valid 0.1 fixtures pass; fake 0.2 declaration is rejected until an approved tag/schema is adopted.
- **Files:** Cargo config, `testkit/fixtures.rs`, `tests/contract_fixtures.rs`, scenario documents, fixture provenance note.
- **Verify:** Schema/serde validation; cancelled fixture grounded in contract + Squint fixture, never invented Rascal provenance; missing/malformed versions rejected.
- **Depends:** F01b. **Commit:** `validate contract fixtures and exact version policy` with the appropriate conventional prefix.

### F09b — Create the independent stdio fake

- **Accept:** A differently named peer independently handles initialize/list/describe/free image call/resources and EOF. Scenarios can report selected contract versions; only supported fixtures claim conformance.
- **Files:** `testkit/main.rs`, `testkit/scenarios.rs`, `tests/fake_wire.rs`, Cargo example target.
- **Verify:** Wire probe for protocol 2025-11-25, pagination, deterministic images, malformed responses and lifecycle; no rmcp/host coordinator reuse.
- **Depends:** F09a. **Commit:** `create the independent stdio fake` with the appropriate conventional prefix.

### F10a — Connect a generic registered MCP tool

- **Accept:** Explicit descriptor starts owned child, negotiates pinned protocol and validates description/version. No namespaced tool-specific shape parsing.
- **Files:** `src/tools/client.rs`, registration module, tool module, lib wiring, `tests/mcp_client.rs`.
- **Verify:** Wrong protocol/version, missing executable, malformed schema, generic names, resource requests and owned-child shutdown.
- **Depends:** F09b/F05a. **Commit:** `connect a generic registered MCP tool` with the appropriate conventional prefix.

### F10b — Render declared action inputs

- **Accept:** Shared neutral Actions picker renders declared fields/image bindings. Internal slots are ordered arrays; version-specific cardinality controls scalar/list wire encoding. Complex fields retain validated JSON editing.
- **Files:** ActionForm, shared picker, picker story, form-state model, form tests.
- **Verify:** Typecheck/browser form tests, required/optional fields, arrays/reorder, wrong cardinality, unavailable unapproved list declaration; no inferred x-squint semantics.
- **Depends:** F10a/F08b. **Commit:** `render declared action inputs` with the appropriate conventional prefix.

### F11a — Guard and persist dispatch

- **Accept:** Under tool reservation, require validated current cost.may_spend == false before invoking MCP. Persist immutable args/request/token and idempotency identity; fail closed on true/missing/malformed/stale cost metadata.
- **Files:** attempt coordinator, attempt DB/migration, run API, `tests/attempt_execution.rs`.
- **Verify:** Count calls at fake: prohibited/stale actions cause zero execution requests. Test bypass via direct API, duplicate tabs/clicks, same ID/different body, busy tool and pre-run rejection.
- **Depends:** F10a/F07b. **Commit:** `guard and persist dispatch` with the appropriate conventional prefix.

F11a is implemented in three verified commits to keep its storage and wire boundaries reviewable:
F11a.1 stores immutable submissions and restart state; F11a.2 persists the SDK-assigned request
identity before transport writes; F11a.3 adds reservation, fresh cost/schema checks and the API.
These preserve the acceptance criteria above; no dispatch is exposed before F11a.3.

### F11b — Preserve terminal contract evidence

- **Accept:** Bind explicit result run identity, persist result then capture verified output/journal/location evidence; separate native/report preservation categories and incomplete saves.
- **Files:** evidence capture module, attempt storage/coordinator, evidence tests, fixture additions.
- **Verify:** Result ID conflict, missing locator, digest mismatch, successful tool + failed save, lost transport and restart; never infer full native preservation.
- **Depends:** F11a. **Commit:** `preserve terminal contract evidence` with the appropriate conventional prefix.

### F12a — Build the shared inspection drawer

- **Accept:** One production drawer has focus/Escape/return behavior, independent scrolling and responsive layout. Same component supports later inspector/export/example content.
- **Files:** Drawer component, story, browser test, component exports.
- **Verify:** Storybook/typecheck; real keyboard/focus, long content, reduced motion, narrow/200% zoom.
- **Depends:** F03b. **Commit:** `build the shared inspection drawer` with the appropriate conventional prefix.

### F12b — Build the shared comparison viewer

- **Accept:** Source/result and baseline/attempt use the same keyboard divider and side-by-side modes; incompatible frames and missing images are explicit.
- **Files:** ImageCompare component, story, comparison model/test, browser test.
- **Verify:** Slider boundaries/keyboard, long labels, missing/incompatible fixtures, Blob URL cleanup; illustrative output not mislabeled as a repair.
- **Depends:** F03b/F08b. **Commit:** `build the shared comparison viewer` with the appropriate conventional prefix.

### F12c — Inspect the selected immutable attempt

- **Accept:** Result workspace and drawer show saved parameters/raw result/call from selected attempt, unaffected by edited draft.
- **Files:** run workspace, inspector content, App wiring, browser test.
- **Verify:** Run → edit draft → reopen inspector; assert saved evidence/call unchanged and labels correct.
- **Depends:** F11b/F12a/F12b/F10b. **Commit:** `inspect the selected immutable attempt` with the appropriate conventional prefix.

### F13a — Stream state invalidation and reconnect

- **Accept:** Bounded authenticated invalidations refresh authoritative snapshots; overflow/reconnect fetches state, and tab closure does not stop work.
- **Files:** event API, browser event client, host integration test, browser reconnect test.
- **Verify:** Multiple tabs, bounded overflow, reconnect, host session replacement, no duplicate dispatch.
- **Depends:** F11b/F06b. **Commit:** `stream state invalidation and reconnect` with the appropriate conventional prefix.

### F13b — Model cancellation without a known run reference

- **Accept:** Cancel once; known resources can finish. Under v0.1 no reference before result is normal: Cancellation requested; final outcome unconfirmed. Keep reservation while child may work, no guessed matching or retry.
- **Files:** coordinator lifecycle, run-status UI, fake cancel scenario, `tests/cancellation.rs`, browser status test.
- **Verify:** Cancel-before-first-progress/no result, no resource, child death, pre-run cancellation, no red false-failure; C4 structured path waits for approved schema.
- **Depends:** F13a/F12c. **Commit:** `model cancellation without a known run reference` with the appropriate conventional prefix.

### F13r — Add an opt-in real Rascal offline smoke

- **Accept:** Ignored test uses explicit real rascal path, temporary workspace/runs, no config/filler/credentials, owned PNG and live no-paid guard. Prefer mask_rect; seam is an additional no-filler case.
- **Files:** `tests/rascal_smoke.rs`, explicit descriptor fixture, smoke instructions.
- **Verify:** `cargo test --locked --test rascal_smoke -- --ignored --nocapture`; validate live description, result/images/fingerprints and unchanged original. Skip honestly if binary absent or contract version unsupported; not a required CI gate.
- **Depends:** F11a/F11b/F12c. **Commit:** `add an opt-in real Rascal offline smoke` with the appropriate conventional prefix.

**Checkpoint M2:** Fake flow: choose → upload → execute → inspect → edit draft → reconnect/restart preserves evidence. No-paid dispatch tests are required. The optional real Rascal check reports its exact revision/version and never weakens safety or version gates.

## M3 — Make experiments reusable

### F14a — Reopen paginated saved history

- **Accept:** History reopens immutable attempts after restart with honest missing-artifact states. No deletion dependency yet.
- **Files:** history DB/API, history UI, integration/browser tests.
- **Verify:** Reopen, pagination, missing bytes fixture and context switching; deletion regressions belong to F17.
- **Depends:** F12c/F13b. **Commit:** `reopen paginated saved history` with the appropriate conventional prefix.

### F14b — Compare saved attempts with a baseline

- **Accept:** Pin a saved attempt by action context and compare saved settings/output, not current draft; no quality scoring.
- **Files:** baseline DB/API, comparison feature, unit/browser tests.
- **Verify:** Context mismatch, setting diffs, slider/side-by-side; pin clearing on deletion is deferred to F17.
- **Depends:** F14a/F12b. **Commit:** `compare saved attempts with a baseline` with the appropriate conventional prefix.

### F15a — Store examples with independent asset references

- **Accept:** Example owns snapshot/reference edges independent of source attempt; opening produces a draft without calling a tool.
- **Files:** example DB/API, migration, example tests.
- **Verify:** Reference edges and snapshot isolation, repeated open produces zero attempts; actual source-deletion regression belongs to F17.
- **Depends:** F14a. **Commit:** `store examples with independent asset references` with the appropriate conventional prefix.

### F15b — Use examples in the shared drawer

- **Accept:** Name/notes, browse/open/save actions reuse shared components; edited form cannot contaminate source snapshot.
- **Files:** example drawer content, story, feature wiring, browser tests.
- **Verify:** Save/open/run new attempt; missing capability visible; no fake previous result reuse.
- **Depends:** F15a/F12a. **Commit:** `use examples in the shared drawer` with the appropriate conventional prefix.

### F16a — Write a bounded evidence archive

- **Accept:** Explicit .tar.gz manifest/evidence writer includes selected inputs, safe relative names, hashes and known setup; reference-only choice explicit. Does not claim portable native replay.
- **Files:** export writer, export model/tests, Cargo dependency update, owned archive fixtures.
- **Verify:** Archive path/symlink traversal, hostile names, credential omission, hash checks, missing asset and independent extraction inspection.
- **Depends:** F11b/F15a. **Commit:** `write a bounded evidence archive` with the appropriate conventional prefix.

### F16b — Copy and download selected-run evidence

- **Accept:** Pipeline drawer copies recorded MCP call/setup and downloads the labeled evidence archive using selected saved attempt.
- **Files:** pipeline drawer content, export API, browser tests, story.
- **Verify:** Edit draft then copy/download; reference-only mode/size list, missing dependency labels, no generated unverified client program.
- **Depends:** F16a/F12a/F15b. **Commit:** `copy and download selected-run evidence` with the appropriate conventional prefix.

### F17a — Delete through reference-aware cleanup

- **Accept:** Preview IDs/reclaimable bytes; transactionally remove selected idle references and clear deleted baseline pins, never originals/shared example assets.
- **Files:** cleanup DB/API, cleanup tests, attempt/baseline integration.
- **Verify:** Now test source-run deletion preserves examples, baseline clears, shared assets survive, stale preview/concurrent example/export, active run refusal and deletion failure.
- **Depends:** F14b/F15a/F16a. **Commit:** `delete through reference-aware cleanup` with the appropriate conventional prefix.

### F17b — Expose explicit cleanup controls

- **Accept:** Disk usage and cleanup selection explain reclaimable/shared/tool-owned bytes; UI never recursively cleans opaque tool storage.
- **Files:** cleanup UI, story, browser tests.
- **Verify:** Delete source run then open example; baseline deletion; failure UI and recalculated preview; no automatic history expiry.
- **Depends:** F17a/F15b. **Commit:** `expose explicit cleanup controls` with the appropriate conventional prefix.

### F18a — Implement confirmed host shutdown

- **Accept:** Active shutdown requires an explicit confirmed request, cancel/wait, and a separate force request for unresponsive owned children. No SDK implicit kill timeout.
- **Files:** host shutdown module, control wiring, coordinator wiring, shutdown tests.
- **Verify:** Unconfirmed request refusal, child refusal/EOF, explicit force, terminal evidence preservation; no unrelated PID termination.
- **Depends:** F13b/F05b. **Commit:** `implement confirmed host shutdown` with the appropriate conventional prefix.

### F18u — Present active shutdown confirmation

- **Accept:** Quit explains active work; declining leaves it running. Confirming shows cancel/wait, with explicit force quit only when needed.
- **Files:** confirmation UI, App wiring, shutdown API client, browser test.
- **Verify:** Decline, confirm, waiting and force states against the host; keyboard/focus behavior.
- **Depends:** F18a/F12a. **Commit:** `present active shutdown confirmation` with the appropriate conventional prefix.

### F18b — Verify the complete development workflow

- **Accept:** dev-fake script uses isolated roots and correct versioned fake; README lists actual commands. Release-style binary launches embedded current UI without JS.
- **Files:** `scripts/dev-fake.ts`, full-workflow browser test, README, milestone report, CI workflow if remote exists.
- **Verify:** Complete command matrix, manual Safari smoke, packaged-source check and two clean UI builds. Record blocked checks and optional Rascal result separately.
- **Depends:** F17b/F16b/F18u. **Commit:** `verify the complete development workflow` with the appropriate conventional prefix.

**Checkpoint M3:** The whole foundation runs against real local processes/files and real browsers. Capture test results/screenshots. This is a development build, not public release or full AC01–AC22 completion.

## Later milestones and their gates

| Milestone | Work | Gate that applies only here |
|---|---|---|
| M4 — Real tool entry/setup | Discovery, Rascal first, Squint next, three launch paths | Implement the early H1 agreement; production installer/registration subgates, C1 unconfigured discovery, R1/S1 evidence |
| M5 — Full tasks and operations | Metadata-driven grouping/readiness, native dependencies, supported CLI/bundle replay | C1/C2/R3; R3 is scheduled in Rascal M10 but not implemented; actual action coverage, no private-layout parsing |
| M6 — Paid runs and reports | Estimate/cap sessions, known-run cancellation, embedded tool pages | C1/C4 spending + association; C3 report readiness/isolation/preservation |
| M7 — Distribution and full acceptance | Library relocation, packaged macOS support matrix, signed/checksummed install path, release verification | Tool relocation dependencies, release source/signing decision, all applicable AC01–AC22 |

M5/M6 are independently gated feature groups; one does not automatically block the other.

## v0.3 client — proposed task list

**Status: approved for implementation, 2026-11-01.** This section specifies the next M4 setup and M6
paid/report slices; it does not mark those milestones complete or replace M5's operations work.
The coordinator owns the contract and sibling changes. Source reviewed: table-manners branch
`v0.3`, commit `e503a238ad6cc833834891c2c2cece7152b08bfc` (coordinator rulings applied after
two review rounds against `61001dd`): CONTRACT.md 3.1, 3.2,
4.4–4.6, 5 and image profiles; schemas/core/estimate.json and referenced schemas; MAPPING's
v0.3 table. Rascal M12 and Squint M9 supply the real peers. No sibling files are edited.

### Scope and host decisions

- Readiness and setup come from `describe`, including unconfigured tools. `present` or `unknown`
  credentials are not described as verified, but can belong to a `ready` action. Description
  readiness describes the action as configured, not every parameter variation. A call omitting
  an optional filler can still run; only requirements the call needs can refuse it. Paid calls use
  estimate readiness, while free calls can receive a structured no-run `not_ready` refusal.
  The host never validates a key speculatively or executes setup prose.
- Estimates are control requests, never attempts. The host prices the exact prepared arguments,
  including images and `max_spend_usd`, then sends the tool's opaque binding in
  `_meta["table-manners/estimate"]`. It does not implement the tool's digest recipe.
- Wafflery requires a fresh allowed estimate and a positive user-selected per-call cap before
  paid dispatch, even though binding and lowering the configured cap are optional in the contract.
  Every paid schema must declare `max_spend_usd` as number with `exclusiveMinimum: 0`.
  Zero means do not run; a larger-than-configured cap is invalid, never silently clamped.
  An allowed estimate is not a reservation or a guarantee that the session still has budget.
- Configuration is selected only through `serve --config <path>` or the tool's default lookup.
  No config-file editor/parser or key store. Only minimal process environment and credential
  and non-secret `env` variable names from declared requirements reach the child. Changed
  config/credentials require
  an explicit restart, a visibly new session and fresh estimates. Old unresolved spend remains.
- Both scalar and `many: true` image slots use the existing owned image library. Optional empty
  slots are omitted; populated list slots send nonempty arrays in the user's chosen order.
- Reports are tool-owned viewers/documents launched from declared argv without a shell. Only
  runs with a usable local directory offer them. Viewer failure never changes a completed run.
  The host owns viewer lifetime and isolates report content from its authenticated application.
- Keep MCP 2025-11-25. Adopt exactly contract `0.3.0`, removing live v0.2 acceptance and
  `cost.needs_env` handling at cutover. Saved older evidence remains readable; replay requires
  a currently supported description. No dual-version dispatch or private-tool fallbacks.

### M4a — v0.3 setup, sessions and image inputs

**V01 — Establish independent v0.3 peer fixtures. Complete.**
Verified in [the execution checkpoint](docs/plans/v03-execution.md). Add schema-valid descriptions for ready,
needs-setup and unavailable actions; session/estimate/refusal fixtures and differently named fake
tools. Script the reserved estimate control with deterministic, zero-provider behavior. Initially
exercise these as independent wire/fixture tests; the live host still refuses v0.3 and paid calls.
Validate against the exact reviewed schema snapshot, not host-generated structures.
Files: testkit fixtures/scenarios, wire tests, fixture tests, testkit README (about five).
Verify invalid readiness, omitted positive-cap declaration and malformed estimate are rejected.
Depends: task-list approval (received). Commit: `test: define the v0.3 independent peer`.
V01 keeps the live v0.2 host unchanged and tests against an archived copy of the reviewed v0.3
schemas. Split its work into V01a (owned fixtures/schema validation) and V01b (independent wire
controls) so each commit stays reviewable. The schema archive is generated input, with source
revision and license recorded; V02 removes it when the exact dependency supplies those schemas.

**V02 — Cut over contract validation deliberately. Complete.** Verified in
[the execution checkpoint](docs/plans/v03-execution.md). Pin exact `table-manners` v0.3.0,
update validators/session parsers and fake default together; remove `needs_env` and reject live
v0.2 declarations. Recognize reserved `estimate` without putting it in the Actions picker.
Reject a paid description if the MCP tool list lacks reserved `estimate`. Keep the offline
dispatch guard intact. Update integration inventory and runtime docs.
Verify unconfigured discovery, invalid paid schemas, old-version refusal, old saved evidence
display and current offline calls. This coordinated version cutover may exceed five authored
files; the known sites are tools/mod.rs, tools/session.rs, UI api/tool-session.ts and workspace.ts,
actions/model.ts, ToolSetup and its stories, testkit fixtures/main/scenarios and their tests.
Enumerate the final set before coding and review schema/parser/fake changes together.
Depends: V01. Commit: `feat: adopt the v0.3 contract`.
V02 merged with an authorized temporary sibling path and CI checkout at e503a23.
The coordinator confirmed crates.io publication on 2026-11-01 and requested the follow-up below.

**V02-publication — Use the published v0.3.0 contract. Complete.** Pin crates.io `=0.3.0`, refresh only
that package's lock entry, remove the temporary sibling checkout from CI and update the active
source inventory. Compare published schemas with reviewed e503a23, then run the full matrix.
No sibling edits or contract changes. Rollback restores the reviewed path/CI checkout together;
no stored data or wire-version change. Commit: `build: pin published table-manners v0.3.0`.

**V03 — Display readiness and actionable setup. Complete.** Replace environment-name-only setup with
per-action requirements and distinct missing/present/valid/invalid/unknown states. Reuse the setup
drawer, keep secrets out, and distinguish unavailable from configurable actions without treating
an unvalidated present key as invalid. Do not blanket-disable `needs_setup`: it describes the
configured action; changed arguments may omit the missing optional dependency. Show call-specific
estimate readiness when available and handle free-call `not_ready` without inventing parameter
rules. Refresh description/session through control requests while
work runs. Preserve selection/draft on refresh. Deliberately retain conservative invalidation:
a readiness change changes the description revision and invalidates prepared calls/quotes, but
not editable inputs. Paid dispatch requires the call-specific estimate to be ready; the tool
remains authoritative for which requirements a particular free call needs.
Files: action model, ToolSetup, its story, API types, browser coverage (about five).
Verify empty config, usable free action beside blocked paid action, present credential, rejected
credential and successful refresh in Chromium/WebKit. Verify `present` to `valid` retains draft
inputs and changes the revision; paid quote-invalidation coverage joins V11/V13 once estimates
exist. Verify an unconfigured optional filler can be
omitted without blocking the free call, and needed missing requirements yield `not_ready`. Depends: V02.
Commit: `feat: explain action readiness in tool setup`.

**V04a — Define the minimal child environment. Complete.** Document an explicit host allowlist in
ENGINEERING.md: HOME and optional XDG_CONFIG_HOME/XDG_DATA_HOME/XDG_CACHE_HOME for tool default
lookup; TMPDIR, LANG/LC_ALL, and a fixed system PATH (`/usr/bin:/bin:/usr/sbin:/sbin`). Never
inherit the whole environment. Forward both `credential` and non-secret `env` requirements when
their names are environment
identifiers (`[A-Za-z_][A-Za-z0-9_]*`); never override this base or forward WAFFLERY_*, DYLD_* or
LD_*. Other/excluded names remain valid displayable requirements (including unknown credentials
held elsewhere), not invalid descriptions; they simply are not forwarded. Copy permitted values
only from the host's inherited environment; persist names, never values, including env values.
Document that external environment changes require relaunching the host, not just the tool.
Files: environment helper, spawn wiring, environment tests, ENGINEERING.md.
Verify default config lookup under HOME/XDG, missing key, loader-variable refusal, unrelated
secret exclusion, non-secret env forwarding, non-identifier unknown credentials and no values in
logs/exports. Depends: V02.
Commit: `feat: define isolated child environments`.

**V04b — Select configuration and discover credentials. Complete.** Add an optional config path to explicit
registration; reject a conflicting existing `--config` argument and pass path as separate argv.
The path is chosen at registration; changing an existing registration remains a stopped-host
operation and needs an explicit replace/update path, not silent append-only registration reuse.
Never read the config. Probe via `serve [--config <path>]` under V04a's minimal environment,
call MCP `describe`, close that no-provider probe, then start the serving session once with the
credential/env variables it declared. Do not invent CLI `describe --config`. The probe's ledger is
not a user session. If the final description adds undeclared credential/env names, show incomplete
setup; no bootstrap loop or automatic restart. Values stay outside descriptors and run evidence.
Files: registration, probe helper/client wiring, registration tests, probe tests, README.
Verify paths with spaces, default lookup, replace refusal while host runs, probe cleanup/failure,
config-dependent requirement names and no paid work in either startup. Depends: V04a.
Commit: `feat: configure and probe tool registrations`.

**V05a — Replace idle tool sessions atomically. Complete.** Give the coordinator a swappable owned-client
slot; submit, cancel, reap, close, preserving-tools, session and snapshot use one consistent
client generation. Reject replacement during active or uncertain execution; use the existing
cancel/stop flow first. Confirmed child exit permits a new session without pretending that old
unknown results/spend were settled. Terminal recorded unresolved spend never permanently blocks
restart. Stop the owned child before starting its replacement; no duplicate serving process.
Repeat V04b's probe with the selected config on every deliberate restart, since edited config
contents can change requirement names. Failed replacement leaves an explicit disconnected state;
stale client references cannot dispatch.
Files: coordinator/session lifecycle, host API, lifecycle tests, race tests, ENGINEERING.md.
Verify submit-versus-restart races, stuck child, initialization failure and retained old evidence.
Depends: V04b. Commit: `feat: replace idle tool sessions atomically`.

**V05b — Present deliberate session restart. Complete.** Add setup restart confirmation that explains the
fresh ledger, shows the new UUID and invalidates descriptions. Publish the new session boundary
that V09–V12 use to invalidate preparations/estimates when those exist. No automatic
restart for credential, cap or budget errors. Retain the old session's unresolved history.
Files: ToolSetup, API client, story, browser tests, README.
Verify confirmation/cancel, active-work refusal, disconnected state and new session identity.
Depends: V03/V05a. Commit: `feat: present deliberate session restart`.

**V06a — Bind list-valued image inputs. Complete.** Generalize materialization, schema validation and cleanup
to ordered arrays for `many: true`, preserving scalar behavior otherwise. Every item carries its
own immutable bytes, fingerprint and coordinate reference. Enforce existing per-file limits and
bounded aggregate request limits before copying; cleanup handles all array paths and only owned files.
Files: attempt binding, materialized cleanup, binding tests, fixture scenario, ENGINEERING.md.
Verify array order, duplicate file identities, scalar/list mismatch, optional omission, empty-list
refusal, fingerprint mismatch and cleanup after partial failure. Depends: V02.
Commit: `feat: bind ordered image lists`.

**V06b — Verify list-image result evidence. Complete.** Replace evidence.rs's first-item-only input match
with per-slot checks of role, identity and order. Completed results must account for the complete
submitted list. Non-completed results can legitimately have empty/partial input evidence: match
reported entries as an ordered subsequence, reject additions/changed identities or roles, and
record missing evidence as a notice without changing the tool's outcome. Handle repeated identical
images by position/count rather than deduplication; do not infer the position of indistinguishable
partial entries. Required images/locations containers still follow the result schema.
Files: evidence.rs, evidence tests, fake result fixtures, testkit README.
Verify two different references, duplicates, wrong roles/order/count and empty/partial failed,
invalid or cancelled results. Depends: V06a. Commit: `fix: associate every list image with result evidence`.

**V07a — Preserve image selections and coordinates. Complete.** Encode declared list cardinality and omit
empty optional slots. Store frame metadata with each selected occurrence so reorder/remove never
separates it from its image; restore ordered selections from immutable saved inputs. Enforce the
host's count and aggregate-byte limits before submission.
Files: action model, selection helper, unit tests, execution notes.
Verify duplicate identities with different frames, reorder/remove, optional omission, required
empty refusal, aggregate limits and saved-input restoration. Depends: V06a/V06b.
Commit: `feat: preserve ordered image selections`.

**V07b — Edit and restore image lists in the workspace. Complete.** Wire list-capable ImageInput into
ActionForm and Workspace with per-image coordinate controls. Saved examples and history drafts
retain order and coordinates; saved-image insertion appends to lists. No tool-specific slot logic.
Keep input scrolling and the Run footer stable. This cohesive UI slice includes component/form/
workspace wiring, a story, browser coverage and generated assets; it is larger than the usual
five-file budget to avoid an intermediate form that loses per-item coordinates.
Verify several references, keyboard reorder/remove, duplicate images with different coordinates,
optional empty slots, history and saved-example round trips. Depends: V07a.
Commit: `feat: edit list image slots`.

**Checkpoint V-A: Complete.** [Verification](docs/plans/v03-setup-checkpoint.md). V01–V07, full matrix and review. Offline actions, setup and list inputs work
against the independent peer; paid dispatch still refuses. Record exact contract/peer revisions
and verification in a checkpoint document. This supplies M4's readiness/session portion, not
automatic discovery, production installation or the full three first-install journeys.

### M6a — Estimates and bounded paid execution

**V08 — Preserve no-run refusals without replay. Complete.** Every new attempt gets a fresh correlation
id; keep attempt id, submission id and client_run_id aligned. A definitive pre-allocation JSON-RPC
error records a terminal no-run refusal with zero spend; an explicit corrected retry creates a
new attempt, optionally linked by `retry_of` on Attempt, not a new field on the strict Submission.
The contract permits released-id reuse, but this host deliberately does not need identity
splitting to use it. Test release/reuse in the independent wire peer separately. A corrected paid
retry needs a new preparation and estimate; never carry authorization from the failed attempt.
Never release the original owner's claim on `client_run_id_reused`: reconcile a non-null run
against that original attempt, and treat null as no permission to resend. A JSON-RPC error after
a known run reference is out of contract: reconcile it; do not label it no-run or released.
Validate `not_ready` as -32602 with run null and missing/invalid requirements; it releases the
claim and creates no tool run. Display setup guidance, not an in-run credential error.
SDK timeout/disconnect is uncertainty, not a definitive refusal. Cancellation before allocation
keeps the claim; a new attempt still uses a new id. Never automatically replay.
Files: association/coordinator, attempt observations, refusal tests, fake scenarios, recovery tests.
Verify corrected fresh-id retry, wire-level released-id reuse, duplicate null/non-null reference,
known-run JSON-RPC error, not_ready claim release, queued cancellation and uncertain disconnect. Depends: V02.
Implementation slices (each reviewed and full-matrix verified):
- **V08a: Complete.** Preserve definitive pre-run RPC refusals and zero spend; test corrected fresh-id
  attempts and wire-level released-id reuse. Commit: `fix: preserve definitive no-run refusals`.
- **V08b: Complete.** Reconcile reused claims and known-run RPC errors; test null references, queued
  cancellation and uncertain disconnect without automatic replay. This slice also fixes the
  receive adapter's progress metadata handling, exposed by the known-run RPC regression; the
  six authored implementation files keep that boundary, recovery and independent peer coherent.
  Commit: `fix: reconcile claimed attempts without replay`.
- **V08c: Complete.** Display general no-run refusals and verify explicit UI retries use fresh identities
  while transport uncertainty retains idempotent submission identity. Commit: `feat: explain no-run refusals in the workspace`.
Rollback later slices before earlier ones; no library migration.

**V09 — Prepare immutable calls without creating attempts.** Extract a bounded prepared-call
resource holding exact arguments and owned materialized inputs before estimation. Bind it to
tool/session/declaration and authenticated workspace. Invalidating a draft releases only its
unclaimed temporary inputs; expiration, abandoned drafts and host startup clean these safely.
The preparation id becomes the attempt id: paths remain under `execution/<id>` and never move
or change between estimate and dispatch. Bound retained preparations by count and total bytes.
Dispatch transfers ownership to an attempt without rereading mutable source files. Estimates
and abandoned preparations create no attempt/history/example records.
Files: preparation module, materialization, coordinator seam, preparation tests, ENGINEERING.md.
Verify changed source bytes, concurrent drafts, expiration, ownership transfer and cleanup races.
Depends: V06a/V06b/V08. Implementation slices (each reviewed and full-matrix verified):
- **V09a: Complete.** Extract immutable prepared inputs with explicit ownership transfer to an attempt.
  Abandoned/failed preparation cleans only its own directory; submission remains owned across
  requester cancellation. Commit: `refactor: prepare exact calls before dispatch`.
- **V09b: Complete.** Retain bounded preparations in the coordinator; enforce identity, session/declaration,
  expiry, invalidation, restart cleanup and atomic consumption. The six implementation files
  keep lifecycle ownership and restart integration in one reviewable slice.
  Commit: `feat: retain bounded draft preparations`.
- **V09c:** Add authenticated preparation controls and conservative startup cleanup, with API and
  crash-leftover coverage. Commit: `feat: expose owned draft preparations`.
Rollback V09c before V09b before V09a. These slices do not change the shared contract; preparations are temporary
host resources, not attempts or library examples.

**V10a — Exempt estimate controls from the run transport gate.** Exempt exactly reserved
`describe` and `estimate` from RecordedTransport's run permit lookup/consumption. All action
calls still require their persisted permit. Neither control writes attempt evidence or touches
run-progress association. Replace ENGINEERING.md's old busy-estimate policy: controls remain
responsive outside the execution reservation; v0.3 estimates never create runs.
Files: dispatch_transport.rs, transport tests, fake control scenarios, ENGINEERING.md.
Verify an estimate while a run permit is staged but not written leaves that permit intact and
both calls succeed; arbitrary tool names cannot bypass the gate. Depends: V02.
Commit: `fix: keep estimate controls outside the dispatch gate`.

**V10b — Request and validate bound estimates.** Call reserved `estimate` with `{action, arguments}`
from V09, outside the execution permit. Hold a preparation lease through the control call so
expiry or draft invalidation cannot delete inputs while the tool reads them; an expired or
invalidated preparation still cannot dispatch afterward. Use a separate 30-second estimate deadline for image
digests rather than the ordinary five-second control timeout. Expiry of that deadline gives no
quote and no automatic retry. Bound concurrent estimate requests and serialize their ledger
observations with session reads so late responses cannot overwrite newer observations. Validate
structuredContent with estimate.json and verify
tool/action/session attribution (including nested session identity/tool), allowed/refused
consistency and effective config/model. MCP estimates must have a non-null matching session;
CLI estimates with null session/session_id never authorize serving calls. Config identity includes
built-in defaults. Accept readiness-refused unpriceable estimates with worst_case_usd null (and
config_identity null when config is missing); display unknown cost, never zero. The positive-cap
and numeric worst-case checks below are authorization requirements, not grounds to discard valid
readiness refusals. Require a finite positive effective cap equal to
the submitted user cap within tolerance; null or silent clamping is invalid for this request. An
allowed estimate must be
ready, with worst case within its cap and any finite remaining session budget. Validate session
arithmetic and continuity through the same validator as session reads. Check the 24-hour expiry
upper bound from host receipt time (the estimate has no issue timestamp), and reject expired quotes.
Treat the binding
as opaque; keep it server-side with the prepared call. Use 1e-6 USD comparison tolerance.
Reject mismatched/expired observations, update the displayed
ledger without adding an attempt, and bound pending requests and retained preparations.
Files: estimate service, control API, API routing, estimate tests, fake estimate scenarios.
Schema-rejected estimate arguments produce plain -32602. Other wrong requests produce -32602
with reason invalid_arguments, run null and error {code usage|input, retryable false, reason};
validate that shape. Missing prerequisites instead return an estimate refused for readiness.
All error/non-estimate responses leave no quote and no attempt. The tool excludes return_images
and image_blocks from its binding; keep the binding opaque and test those delivery flags without
reimplementing its recipe or relaxing the host's exact prepared-wire comparison.
Verify responsive estimates during held runs, malformed/wrong-session replies, refused readiness/
cap/budget, expiration and zero history/provider side effects. Include schema-valid contradictory
fixtures: null/lower/oversized cap, allowed but not ready, worst case above cap/remaining budget, nested
session/tool mismatch, CLI null sessions, unpriceable readiness, defaults-based config identity,
invalid_arguments vs plain invalid params, control timeout and expiry over 24 hours. Depends: V09/V10a.
Commit: `feat: request bound cost estimates`.

**V11 — Present the cap and exact estimate.** Add one positive cap control backed by the declared
`max_spend_usd` field, avoiding a duplicate generic form field. Show worst case, effective model,
cap, remaining session budget and unresolved spend; distinguish uncapped null from zero. Require
an explicit cap before paid Run. Draft edits (including cap, ordered images or parameters),
declaration change, a new session id and expiry invalidate the estimate. Ledger amount changes
alone do not invalidate it; the tool still checks current budget when called. Re-estimating never executes.
Files: cost UI, its story, form wiring, API client, browser tests (about five).
Verify zero/negative/nonfinite caps, stale replies after edits, above-config refusal without
clamping, unknown rather than zero unpriceable cost, needs-setup and an allowed estimate that
later loses budget while its readiness/declaration stays unchanged. Depends: V03/V10b.
Commit: `feat: review cost and cap before paid runs`.

**V12 — Enforce the paid guard on the server.** Replace the offline-only refusal only after
V08–V11 pass. Direct API requests must supply a live owned preparation with matching declaration,
session, positive cap and semantically validated allowed unexpired estimate from V10b. Atomically
consume it once and dispatch
exactly the priced arguments with `_meta["table-manners/estimate"]` beside correlation/progress
metadata. Bind the estimate token in the transport permit too: the write boundary rejects absent
or substituted paid bindings. Neither a client-supplied amount nor an invented binding authorizes a
call. Preserve
the recorded binding, cap and effective estimate in selected-attempt evidence/export; no secrets.
Files: dispatch guard, transport, evidence, guard tests, recorded-call tests (about five).
Verify bypass attempts, duplicate submission, changed files, session restart, expired quote, paid
retry requiring new preparation/estimate and
exact wire args/meta. Fake paid actions simulate spend without network/provider calls.
Depends: V05a/V05b/V08/V09/V10a/V10b/V11. Commit: `feat: enforce bound paid dispatch`.

**V13 — Explain paid failures and retained spend.** Map `stale_estimate` to review/re-estimate,
`credential` (provider 401/403 during a run only) to setup, `cap` to explicit cap review and
`budget` to the session ledger. A larger
than configured cap remains invalid/usage. Never auto-retry or restart; a new estimate requires
a new explicit Run. Refresh readiness after credential outcomes. Keep cancelled/failed actual
and unresolved spend visible and distinguish no-run refusals from allocated run errors.
Files: result/error presentation, setup refresh, stories, error tests, browser tests (about five).
Verify pre-run missing/invalid requirements return not_ready with no tool run; in-run fake
auth rejection costs zero; unresolved provider response costs worst case; two allowed
estimates with unchanged credential state can execute sequentially with the second later refused
for budget; separately test present-to-valid invalidates the second quote without dispatch; stale
estimates spend zero.
Depends: V12. Commit: `feat: explain paid execution failures`.

**Checkpoint V-B:** Full matrix, review and fake paid journey from cap/estimate through run,
cancel, history and evidence export. Keep one active execution per tool: a second submission is
busy, while two earlier estimates
can outlive a sequential budget change when readiness stays unchanged. Test readiness-driven
invalidation separately. Verify controls remain responsive, and restart invalidation. No real
credentials in mandatory
tests and no claim of provider verification until V18. Record this checkpoint separately.

### M6b — Tool report viewers and documents

**V14 — Own render command execution.** Resolve declared render argv from the selected saved
run, substitute `{run.dir}` and `{frame_ancestor}` only as whole argv elements, and spawn without a
shell. Require command[0] to equal description tool.name; replace it with the exact registered
serve executable. Reject absolute names, other programs and lookup through PATH. A render must
not modify run.dir; verify the fake obeys this without implying an OS sandbox. Require
an existing authorized local run directory; null/missing directories give an unavailable view,
not a guessed path. Reject malformed declarations, bound diagnostics/output and retain child
handles. Report processes receive minimal environment, not provider credentials or browser tokens.
Files: render module, host render API, routing, fake render executable, render boundary tests.
Verify spaces, unknown placeholders, missing/copied directories, shell metacharacters and
attempt selection independent of the edited draft. Depends: V02/V04a.
Commit: `feat: launch declared run renderers`.

**V15 — Manage server readiness and lifetime.** Pass the host's exact origin through the declared
`--frame-ancestor` argv placeholder. Accept exactly one stdout URL line with http, literal
127.0.0.1 and a valid port; reject credentials, redirects outside that origin and extra stdout.
Honor `ready_timeout_s` up to 60 seconds. Keep at most one viewer per run. Use an in-memory
five-minute idle timeout refreshed by visible
inspection activity; abandoned/disconnected tabs eventually release the child, without a durable
lease system. Keep stdin open while viewing; close it on viewer close,
replacement or host shutdown, then use SIGTERM for an unresponsive owned viewer. Own a separate
viewer process group and signal only
that group; test descendant cleanup. Clean EOF/SIGTERM must exit zero. A stuck viewer retains
its run slot as stopping/needs-force and cannot be reopened until confirmed exit. Retain existing
explicit-force policy if it still will not exit. Failure never alters the run or spend.
Files: server renderer, lifecycle integration, fake server scenarios, lifecycle tests, ENGINEERING.md.
Verify delayed readiness, early/nonzero exit, malformed/multiple lines, timeout, stdin close,
zero exit on EOF/SIGTERM, stuck slot, owned process group, duplicate opens, idle/disconnected-tab
cleanup and exact framing origin. Depends: V14.
Commit: `feat: manage local report server lifetime`.

**V16 — Isolate document outputs.** A declared document renderer writes one self-contained
file outside run.dir, with images/styles inline, then prints only its absolute path and exits
zero. Read exactly that returned regular file through a narrowly scoped per-render output grant;
its location is tool-chosen, not restricted to run.dir or the image output roots. Apply a bounded
read and explicit timeout; reject symlinks, non-regular files, in-run outputs and path/open races.
Serve an immutable copy on a separate report origin, never the authenticated app or file://.
Do not serve adjacent files, recursively copy folders or delete the tool's source output. A file
needing external assets is nonconforming; display a report error while preserving the run.
This is viewing, not the still-gated C2 portable export.
Files: document renderer, isolated content server, fake document scenarios, boundary tests, docs.
Verify outside-run output, in-run rejection, missing/nonzero output, non-regular/symlink refusal,
active HTML isolation, byte limit, no neighboring-file reads and a copied run directory.
Depends: V14. Commit: `feat: isolate rendered documents`.

**V17a — Add exact-origin report framing.** Replace the app's `frame-src 'none'` only with
permission to frame a host-owned report wrapper. That wrapper contains no tool HTML and sets a
per-response CSP permitting only the selected report's exact origin. Both ancestors use the
host origin, matching the server's exact frame-ancestor contract. Tool/document content stays on
a separate origin, never on the authenticated app origin. The trusted wrapper iframe is not
sandboxed, preserving its host origin for the ancestor check. Only its inner report iframe uses
sandbox="allow-scripts allow-same-origin"; no top-navigation, forms, popups or download flags.
The report must be cross-origin from the wrapper; never allow these flags for same-origin tool HTML.
Do not grant a loopback-port wildcard to arbitrary report destinations. Use a short-lived, single-use,
scoped viewer grant in the wrapper URL, not the API bearer or bootstrap secret; bind it to
the selected saved run and do not log it. Set Referrer-Policy: no-referrer on the wrapper. Define
route-specific frame headers instead of inheriting
X-Frame-Options DENY everywhere. Reject unapproved redirects/navigation.
Files: API headers/routing, wrapper handler, frame boundary tests, ENGINEERING.md.
Verify real nested-frame CSP in Chromium/WebKit; malicious report scripts cannot reach parent
state, authenticate API requests or navigate the top-level app. Depends: V15/V16.
Commit: `feat: isolate report frames by exact origin`.

**V17b — Embed reports in the shared run drawer.** Offer tool-declared views on a saved run;
show readiness/loading/error and open V17a's wrapper. Close/replacement releases the viewer;
visible activity refreshes its idle timeout. Keep raw evidence/comparison usable on start failure.
Files: report drawer, API client, story, browser tests, README.
External navigation/popups are blocked; this slice offers no viewer-controlled external launch.
Verify frame load, blocked external-link behavior, close/reopen/replacement, background expiry and
startup failure. Dependencies: V17a. Commit: `feat: inspect tool reports in the run drawer`.

**Checkpoint V-C:** Full matrix and review with real fake-peer report processes, not static UI
fixtures. Confirm shutdown ownership and no run-status mutation on report failure. Native Safari
gets a focused report/frame/restart walkthrough; report it separately if pending. Record evidence.

**V18 — Verify the real v0.3 peers.** After Rascal M12 and Squint M9 arrive, use isolated builds
of coordinator-approved revisions and disposable library roots. Verify unconfigured readiness,
config selection/restart, free Rascal execution, Squint list references, bound estimates and both
tools' declared report kinds. Use each tool's scripted provider where supported first. A real
paid probe uses the same cap/estimate flow with an explicit small cap; preserve costs and redact
secrets from evidence. Existing paid-run authorization applies, but no unbounded provider test
or key collection is part of CI. Unsupported scenarios stay explicit outstanding checks.
Files: opt-in Rascal/Squint probes, integration inventory, acceptance/checkpoint docs (about five).
Verify default lookup and dependencies under the fixed PATH; if a real tool needs an excluded
helper path, report the concrete need to the coordinator rather than broaden inheritance silently.
Verify exact wire/evidence with each real peer, then full matrix for any changed host code.
Depends: V-A/V-B/V-C plus published tools. Commit: `test: verify real v0.3 integrations`.

### Approval, sequencing and rollback checkpoints

Mark approved V01–V18 on 2026-11-01 after the two review rounds. The initial proposal was
commit 0033af8. Implementation proceeds in order with the per-task checks below; the coordinator
reviews and pushes local merges. Paid dispatch remains blocked until its complete guard lands.

After approval, use short-lived branches for V-A, V-B and V-C; V08 can follow V02 and V14
can follow V02/V04a independently, while paid dispatch must wait for its complete guard. Each
numbered task uses
its listed commit, starts with failing behavioral tests, includes matching docs and passing
targeted checks. File lists are budgets, not excuses to omit wiring/tests: split an oversized
task into named sub-tasks before implementing it. Each checkpoint gets a final
`docs: record v0.3 <setup|paid|reports> verification` commit with exact checks/limits. Merge a
coherent checkpoint only after the full matrix above and review; the coordinator pushes, never
this session. First hosted CI is independent evidence, not implied by a local pass.

- **Before V-A:** stable rollback is the current v0.2/offline host. Revert version-cutover
  dependents together, never combine old validation with new wire fields. Later v0.3 tools will
  correctly be refused by that older host; do not add a fallback adapter.
- **Before V-B:** stable rollback is V-A with paid dispatch refused. Disable/stop new paid work
  first, let active work settle or explicitly cancel, and retain recorded/unresolved spend.
  Revert paid dependents newest first. Never delete attempts to make accounting appear clean.
- **Before V-C:** stable rollback is V-B without embedded reports. Close owned report children
  before reverting; preserve runs and tool-owned report files. V-C can land before V-B if its
  dependencies pass; rollback then returns to V-A instead.
- Use disposable data for all development checks. Put optional new linkage/estimate evidence on
  Attempt, not strict Submission; prove older
  evidence stays readable and document binary downgrade limits. If storage needs a database
  migration, name and test it in that task, back up real data before adoption and refuse unsafe
  downgrade. Do not restore an old ledger as a purported continuation of a new tool session.

### Coordinator decisions incorporated after review

All three questions are settled in table-manners `e503a23` and integrated above:

- **Q-R:** render command[0] is exactly tool.name; execute the registered serve binary. No absolute
  command[0], other program or alternate PATH installation. Placeholder substitution is whole-argv.
- **Q-E:** estimates never create runs. Input-schema rejection is plain -32602; other invalid
  arguments use the structured invalid_arguments refusal with run null and nonretryable usage/input
  error. Not-ready calls return an estimate refused for readiness, with null cost if unpriceable.
- **Q-D:** one self-contained document file outside run.dir; only its returned file is read. No
  adjacent assets or run-directory mutation. Server renderers exit zero on stdin EOF/SIGTERM.

The updated baseline also defines not_ready before allocation, kind env, configured-action versus
call-specific readiness, defaults in config identity, null CLI sessions and delivery-only binding
exclusions. These have explicit tasks/tests above. No unresolved coordinator question currently
blocks the host task list. Publication and real-peer delivery remain integration gates.

The minimal environment and one-shot MCP probe are host choices within 4.6. Any new contract
question goes to the coordinator; never edit siblings. Automatic discovery/install, native
operations, portable C2 bundles and batch evaluations remain separate work. Review disposition
is in [the plan review](docs/reviews/2026-11-01-v03-plan.md).


### Coordinated rename — 2026-11-01

The owner approved the canonical name `wafflery`, `WAFFLERY_*` variables, and a new default
data parent `~/Library/Application Support/Wafflery`. No old-name aliases, environment fallbacks,
or automatic library migration. The coordinator owns sibling launcher and shared-contract edits.
The coordinator subsequently moved the checkout to `wafflery` and published it; the procedure
below records the completed rename, not a pending directory move.

Branch `rename-wafflery` starts at `3836cd6`. Commit the mechanical runtime/UI/test rename with
regenerated assets together so the binary, imports and browser protocol stay consistent; this
explicit mechanical rename exceeds the ordinary authored-file budget. Then commit docs/mocks,
current M4 status and the full verification record separately. Run the complete matrix below,
review, and fast-forward main locally. No remote additions, pushes, tags or sibling edits.
Results and the v0.3 readout are in the [rename checkpoint](docs/reviews/2026-11-01-rename-wafflery.md).

Batch evaluations remain after this individual-run release; Pixel/chains/Linux stay deferred.
Installer/download ownership remains H1. Do not use an unsigned fictional release URL in tests
or publish a release as a side effect of completing the foundation.

## Verification and commit strategy

At each task run its targeted checks. At M0/M1/M2/M3 run the checks relevant to code now present:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
bun run --cwd ui typecheck
bun run --cwd ui test:unit
bun run --cwd ui build:storybook
bun run --cwd ui test:browser
bun run scripts/check-ui.ts
cargo build --locked --release --bin wafflery
```

The command matrix above is implemented. The foundation runtime document records its design.
Don't rerun expensive complete
suites after unchanged documentation; broaden checks when code changes or evidence requires it.

Create a short-lived milestone branch after the docs baseline. Each passing task gets a clear
commit as listed. Review staged changes and matching tests before committing; merge coherent
milestones only after code review and their checks. No automatic push or remote PR is required.
Commit generated assets with corresponding UI changes; distinguish them in review statistics.

Rollback a code slice with `git revert` of its commit(s), newest dependent first; never discard
unrelated user work. Tests use disposable library roots. Preserve real library data and database
backups; a newer schema may require the matching newer binary rather than automatic downgrade.
An incomplete task can remain on its branch with an honest status; never commit a false passing
checkpoint to save time. No backward-compatibility shims are added for unreleased APIs.

## Risks and decisions retained

| Risk | Response |
|---|---|
| Environment selection / browser execution | Record actual Node path/version; cached Chromium/WebKit must match the chosen Playwright and launch. Report sandbox limitations, do not bypass them |
| Contract version changes / roots deprecated after current MCP revision | Exact supported contract set and explicit schema upgrade commit; v0.3 tasks above cover list bindings; no MCP revision bump until replacement path-access rules are agreed |
| Shared SDK conceals protocol assumptions | Independent fake wire peer; explicit protocol selection and manual child ownership |
| UI fixtures accidentally become tool-specific logic | Keep fixtures in Storybook/testkit; test differently named tools; validate contract objects |
| Saved evidence is mistaken for full reproducibility | Separate preservation categories and label exports accurately; C2/C3 gates remain |
| Stable tool workspaces grow beyond movable library | Show storage locations separately; cleanup/relocation never guesses ownership |
| Too much infrastructure before a working screen | M1 import loop and M2 execution loop are visible checkpoints; no speculative packages/services |

M0–M3 do not require another product decision. R3 now has a supplier milestone (Rascal M10);
its shared profile still needs agreement because independent Operations is core to the full release.
C3's v0.3 delivery is tracked by V14–V18 against Rascal M12/Squint M9; C2 portable replay still
needs a separate integration checkpoint. R2 remains optional. Do not imply an Actions-only
development build satisfies those release requirements. Product choices remain Q1–Q19.
