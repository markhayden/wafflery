# Wafflery foundation implementation plan

Repository update (2026-11-01): the canonical checkout is `wafflery`; the coordinator has
published main and manages subsequent pushes. The [CI gate](docs/plans/ci.md) runs this plan's
full matrix on `macos-15`; its [checkpoint](docs/reviews/2026-11-01-ci.md) separates local checks
from the first hosted run. Local merges remain subject to review; no automatic pushes or tags.

Status: M0–M3 and V01–V19 complete; M5 v0.4 approved on 2026-11-02.
Current implementation evidence: [M0 checkpoint](docs/reviews/2026-10-31-m0-checkpoint.md) and
[M1 checkpoint](docs/reviews/2026-10-31-m1-checkpoint.md), and
[M2 checkpoint](docs/reviews/2026-10-31-m2-checkpoint.md), and
[M3 checkpoint](docs/reviews/2026-10-31-m3-checkpoint.md). M0–M3 are complete: automated and
packaged-source checks pass, and the owner explicitly confirmed the native Safari checklist.
M4's explicit real-Rascal entry slice is merged on main, including registration and the real
launcher/browser journey. See its [checkpoint](docs/reviews/2026-10-31-m4-entry-checkpoint.md).
The Squint offline entry and published v0.2 integration are also merged, as is the shared
Tool setup drawer with live session spending. M4 remains incomplete: automatic installed-tool
discovery, production installer/registration, and the complete first-install journeys from all
three entry points still need delivery. V01–V19 delivered declared readiness/provider setup,
bound paid execution, reports and the UI fidelity pass.
The [current review disposition](docs/reviews/2026-10-31-plan-review-disposition.md) records
changes and validation limits; it is not implementation verification.
The owner agreed to proceed with foundation planning independently of later tool interfaces.
The owner authorized continuing into real-tool integration on 2026-10-31; the concrete next
slices are in [M4: real tool entry](docs/plans/m4-real-tool-entry.md). Paid runs are also authorized
through owned preparation, bound estimate and cap/session checks. Browser paid Run uses the same owned preparation and estimate.
Publishing and production installation are not authorized by this continuation.

Prior integration checkpoint (2026-11-01): published v0.2.0 and both tools were adopted on main.
See [the v0.2 integration plan](docs/plans/table-manners-v02.md) and [verified checkpoint](docs/reviews/2026-11-01-table-manners-v02.md). The initial v0.1 fixture/version
requirements below describe the completed foundation checkpoint, not the current live allowlist.
The [v0.3 client task list](#v03-client--proposed-task-list) is complete through V19 at `851b850`;
hosted CI run `41438647360` passed on 2026-11-02. The live client requires exactly v0.3 and
enforces paid dispatch through owned preparations, bound estimates and explicit browser Run.
See [execution checkpoints](docs/plans/v03-execution.md). The M5 plan below was approved on 2026-11-02; implementation proceeds in task order.

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
| M5 — Full tasks and operations | v0.4 ports, artifact chaining, invocation/CLI/bundle replay; proposed M5.01–22 below | Owner plan approval; published v0.4 and approved Rascal M10/Squint M10 peer verification; no private-layout parsing |
| M6 — Paid runs and reports | Delivered by V01–V19: estimate/cap sessions, association, embedded pages/documents | Completed v0.3 peer/browser evidence; retained as regression coverage in M5 |
| M7 — Distribution and full acceptance | Library relocation, packaged macOS support matrix, signed/checksummed install path, release verification | Tool relocation dependencies, release source/signing decision, all applicable AC01–AC22 |

M5/M6 are independently gated feature groups; one does not automatically block the other.

## v0.3 client — proposed task list

**Status: complete through V19, 2026-11-02 (`851b850`, hosted CI green).** Originally approved
2026-11-01, this section records the M4 setup and M6 paid/report slices; it does not complete
M4 distribution/discovery or replace M5's operations work.
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

**V09 — Prepare immutable calls without creating attempts. Complete.** Extract a bounded prepared-call
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
- **V09c: Complete.** Add authenticated preparation controls and conservative startup cleanup, with API and
  crash-leftover coverage. Commit: `feat: expose owned draft preparations`.
Rollback V09c before V09b before V09a. These slices do not change the shared contract; preparations are temporary
host resources, not attempts or library examples.

**V10a — Complete: exempt estimate controls from the run transport gate.** Exempt exactly reserved
`describe` and `estimate` from RecordedTransport's run permit lookup/consumption. All action
calls still require their persisted permit. Neither control writes attempt evidence or touches
run-progress association. Replace ENGINEERING.md's old busy-estimate policy: controls remain
responsive outside the execution reservation; v0.3 estimates never create runs.
Files: dispatch_transport.rs, transport tests, fake control scenarios, ENGINEERING.md.
Verify an estimate while a run permit is staged but not written leaves that permit intact and
both calls succeed; arbitrary tool names cannot bypass the gate. Depends: V02.
Commit: `fix: keep estimate controls outside the dispatch gate`.

**V10b — Complete: request and validate bound estimates.** Call reserved `estimate` with `{action, arguments}`
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

V10b execution slices (same acceptance criteria; full matrix and review per slice):
- **V10b1: Complete.** Strict estimate/refusal validation with the shared session validator and independent
  fixtures. No live request or paid-dispatch path yet. Commit: `feat: validate bound estimate observations`.
- **V10b2: Complete.** Bounded control requests, serialized ledger observations and preparation leases.
  A timeout returns no quote but retains input ownership until the response completes or the
  child exits; such leases still count against retention limits. Commit: `feat: own estimate requests through completion`.
  The ownership slice spans prepared.rs/retained.rs, client.rs/estimate control, coordinator
  wiring/service, independent peer wiring/readiness/estimates/held-run behavior and the live
  estimate test. Review these together: splitting the reader lease from timeout and shutdown
  handling would leave an unsafe intermediate control path. It exceeds the usual file budget.
- **V10b3: Complete.** Authenticated estimate API, retained opaque bindings and snapshot publication;
  recheck preparation validity before accepting a response. Commit: `feat: expose prepared cost estimates`.
  This API/lifetime slice includes the retained quote module, expiry and ticket handling, control
  routes/service, snapshot projection and their independent-peer/unit/API coverage. Review the
  joined boundary together so a timeout cannot publish a quote through an unguarded intermediate API.
Rollback in reverse order. V10b1's cohesive validation slice includes estimate module/tests,
module registration, session-validator visibility, the existing MCP transport probe and moving the pinned time dependency
into production with parsing enabled. Later slices connect it without enabling paid dispatch.

**R-path — Complete: grant explicit input access to real Rascal launches.** The coordinator's
`9ee6bc8` update requires `serve --allow-dir` before any caller path can be read. Update
only Wafflery-owned development/test descriptors to grant the existing absolute
`Library/execution` directory containing copied inputs; keep generic registration opaque.
Use one shared argument builder for the development launcher and browser fixtures, and the
same grant in the Rust smoke. Do not grant the home directory or migrate Squint before its
matching change lands. Verify the real offline smoke fails before this update and passes
afterward, then run real Rascal browser coverage and the full matrix. No keys or sibling edits.
Commit: `fix: grant copied input access to Rascal`; rollback the descriptor/helper changes
together (old descriptors will fail against current Rascal). This small integration correction
precedes V11; it does not adopt v0.4 or change MCP roots for the v0.3 client.

**V11 — Complete: present the cap and exact estimate.** Add one positive cap control backed by the declared
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

V11 execution slices (full matrix and review per slice; rollback in reverse order):
- **V11a: Complete.** Typed browser preparation/estimate controls, response attribution, shared readiness
  validation, positive decimal cap parsing and expiry helpers. Keep controls unused by the
  workspace until wiring is ready; no dispatch or history changes. Verify malformed responses,
  unknown costs, identity mismatches, exact request bodies, invalid caps and bounded lifetime.
  Commit: `feat: add browser estimate controls`.
- **V11b: Complete.** Cost component, states, styles and Storybook coverage. Show one cap, effective
  model/configuration, worst case, current allowance and refusal/setup information. Verify
  keyboard behavior, invalid values and unknown versus zero amounts in both browsers.
  Commit: `feat: present cost estimate states`.
- **V11c: Complete.** Wire the component and owned preparation lifecycle into the workspace; remove the
  duplicate generic paid cap field. Invalidate on exact draft/session/declaration/expiry changes,
  ignore stale replies and discard abandoned preparations. Preserve estimates on ledger-only
  updates. Verify live independent-peer cases in both browsers, no attempt on estimate, and
  no automatic re-estimation or paid dispatch. Commit: `feat: review exact draft costs`.
The API slice includes its tests and existing readiness-validator reuse; the component and
lifecycle slices each keep their stories/browser tests alongside their source. V12 owns the
subsequent paid submit wiring and transport/evidence enforcement.

**V12 — Enforce the paid guard on the server. Complete.** Replace the offline-only refusal only after
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

V12 execution slices (full matrix and review per slice; rollback in reverse order):
- **V12a: Complete.** Carry the validated quote in the transport permit; reject absent, substituted,
  unexpected, refused or expired bindings before a wire write, including expiry during evidence
  persistence. Keep the coordinator's paid guard closed. Commit: `feat: bind dispatch permits to validated estimates`.
- **V12b: Complete.** Atomically consume matching retained preparations with their allowed live quotes,
  verify copied inputs and preserve selected quote/cap evidence. Keep ordinary submissions
  unable to authorize paid work. Commit: `feat: consume priced preparations atomically`.
- **V12c: Complete.** Connect paid prepared submission to the coordinator and exact wire metadata;
  exercise independent fake paid calls, bypass attempts, idempotence and evidence/export.
  Commit: `feat: dispatch only owned priced calls`.
- **V12d: Complete.** Enable explicit paid Run only for the current quoted preparation; preserve
  acknowledged attempt identity and require fresh preparation/estimation for another paid
  attempt. Verify edited, expired and refused drafts cannot run. Commit: `feat: run reviewed paid drafts`.
Each slice includes its tests and accurate checkpoint documentation. V12a/b strengthen internal
boundaries without opening paid dispatch; V12c opens only the authenticated prepared-call API,
and V12d connects the reviewed browser flow. No provider call is required for verification.

**V13 — Explain paid failures and retained spend. Complete.** Map `stale_estimate` to review/re-estimate,
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

V13 execution slices (full matrix and review per slice; rollback in reverse order):
- **V13a: Complete.** Independent fake failures, actual/unresolved spend and cancellation reservations;
  refresh host readiness after credential outcomes and verify sequential budget changes separately
  from readiness changes. Commit: `feat: preserve paid outcomes and refresh readiness`.
- **V13b: Complete.** Selected-run failure/spend presentation and focused recovery guidance, with stories,
  unit and browser coverage. Commit: `feat: explain paid outcomes in the workspace`.
- **V13c: Complete.** Complete the fake paid journey through cancellation, history and exported evidence;
  record checkpoint V-B. Commit: `test: verify the paid workflow checkpoint`.
Each slice keeps its relevant tests and accurate documentation; none calls a real provider.

**Checkpoint V-B: Complete.** [Verification](docs/plans/v03-paid-checkpoint.md). Full matrix, review and fake paid journey from cap/estimate through run,
cancel, history and evidence export. Keep one active execution per tool: a second submission is
busy, while two earlier estimates
can outlive a sequential budget change when readiness stays unchanged. Test readiness-driven
invalidation separately. Verify controls remain responsive, and restart invalidation. No real
credentials in mandatory
tests and no claim of provider verification until V18. Record this checkpoint separately.

### M6b — Tool report viewers and documents

For V14–V17, use the approved `work/*.html` mockups as the design reference from the start,
especially the shared drawer, result hierarchy and preview controls. Preserve accessibility,
contract boundaries and tested behavior where the mock is incomplete; record material divergences
for V19 rather than introducing a separate report-viewer visual language.

**V14 — Own render command execution. Complete.** Resolve declared render argv from the selected saved
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

V14 execution slices (full matrix and review per slice; rollback in reverse order):
- **V14a: Complete.** Resolve a declared render from saved attempt evidence, validate its registered
  executable, authorized run directory, whole-argument placeholders and host framing origin.
  Build the minimal-environment command without spawning it. Verify malformed commands, path
  escapes, spaces and literal shell metacharacters. Commit: `feat: resolve saved run render commands`.
- **V14b1: Complete.** Own renderer children and bounded output, with the independent fake renderer and
  process-level tests. Commit: `feat: own declared renderer processes`.
- **V14b2: Complete.** Retain renderer ownership in the coordinator and add authenticated host controls.
  Keep report content unavailable until V15/V16 establish readiness and output isolation.
  Commit: `feat: coordinate saved report processes`.
The first slice cannot execute a render and introduces no route or browser policy change.

**V15 — Manage server readiness and lifetime. Complete.** Pass the host's exact origin through the declared
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

V15 execution slices (full matrix and review per slice; rollback in reverse order):
- **V15a: Complete.** Validate the single stdout URL, exact loopback authority, separate report origin,
  process state and readiness deadline. A later extra line invalidates a ready view.
  Commit: `feat: validate report server readiness`.
- **V15b: Complete.** Supervise readiness in retained views, with real independent HTTP report fixtures and
  bounded startup failures. Keep URLs private until framing exists. Commit: `feat: supervise report readiness`.
- **V15c: Complete.** Add owned process groups, graceful SIGTERM and descendant cleanup while retaining
  explicit-force behavior and safe process identity. Commit: `feat: stop owned report process groups`.
  Split into **V15c1: Complete** (`feat: retain owned report process groups`): a macOS group owner that
  keeps the leader unreaped, checks bounded group observations and signals only its pinned
  identity; then **V15c2: Complete** (`feat: stop owned report process groups`): integrate EOF/SIGTERM/
  explicit-force escalation with descendant fixtures and the existing view lifecycle. Each
  slice runs review and the full matrix. Linux report process groups remain unsupported until
  an equivalent ownership implementation is tested; macOS is the approved initial platform.
- **V15d: Complete.** Add five-minute idle expiry, visible-inspection activity and replacement/shutdown
  lifecycle coverage. Commit: `feat: expire idle report viewers`.
No slice enables tool HTML in the app origin; V17 owns framing and the drawer integration.

**V16 — Isolate document outputs. Complete.** A declared document renderer writes one self-contained
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

V16 execution slices (review and full matrix per slice; rollback in reverse order):
- **V16a: Complete.** Consume the successful document renderer's one path line through a narrow output
  grant, with a 32 MiB immutable read, five-second read deadline and identity/race checks.
  Read no adjacent files and leave the original untouched. Commit: `feat: read declared document outputs`.
- **V16b: Complete.** Serve immutable documents from an isolated loopback origin with restrictive resource
  policy, bounded lifetime and tests for neighboring-file refusal and external dependencies.
  Commit: `feat: serve isolated report documents`.
- **V16c:** Retain document generation/read/content lifetimes in views, enforce a 30-second
  generation deadline, and integrate ready/error/close/idle/shutdown states with independent
  document fixtures. Split into **V16c1: Complete**, the owned document monitor and independent fixtures,
  and **V16c2: Complete**, coordinator lifecycle integration and API regression tests. Review and run the
  full matrix for each; rollback V16c2 before V16c1. Commits: `feat: supervise document report lifetimes`
  and `feat: coordinate document report lifetimes`.
No content is embedded in the workspace until V17's framing checks pass.

**V17a — Add exact-origin report framing. Complete.** Replace the app's `frame-src 'none'` only with
permission to frame a host-owned report wrapper. That wrapper contains no tool HTML and sets a
per-response CSP permitting only the selected report's exact origin. Both ancestors use the
host origin, matching the server's exact frame-ancestor contract. Tool/document content stays on
a separate origin, never on the authenticated app origin. Revalidate the current process/output
when issuing a viewer grant; a cached ready observation can lag a process update. The trusted wrapper iframe is not
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

V17a execution slices (review and full matrix per slice; rollback in reverse order):
- **V17a1: Complete.** Bounded, sixty-second, single-use frame grants scoped to one existing view identity.
  Keep them private to the host lifetime. Commit: `feat: scope one-use report frame grants`.
- **V17a2: Complete.** Revalidate current view targets, issue/consume grants through authenticated controls
  and a trusted wrapper, and apply route-specific frame headers. Verify the real nested frame
  in both browsers before exposing drawer controls. Commit: `feat: isolate report frames by exact origin`.
- **V17a3: Complete.** Extend independent malicious-report fixtures and browser checks for parent/API access,
  redirects, external resources and navigation restrictions. Commit: `test: verify report frame isolation`.

**V17b — Embed reports in the shared run drawer. Complete.** Offer tool-declared views on a saved run;
show readiness/loading/error and open V17a's wrapper. Close/replacement releases the viewer;
visible activity refreshes its idle timeout. Keep raw evidence/comparison usable on start failure.
Files: report drawer, API client, story, browser tests, README.
External navigation/popups are blocked; this slice offers no viewer-controlled external launch.
Verify frame load, blocked external-link behavior, close/reopen/replacement, background expiry and
startup failure. Dependencies: V17a. Commit: `feat: inspect tool reports in the run drawer`.

V17b execution slices (review and full matrix per slice; rollback in reverse order):
- **V17b1: Complete.** Typed private report client and lifecycle ownership. Expose the existing view for a
  saved attempt so reopening inspection can recover a pending close/force state without starting
  another renderer. Keep one ownership manager across drawer mounts; serialize open/replacement/
  close, retain late-open ownership after dismissal, and reject malformed observations/frame
  URLs. Lost acknowledgements remain uncertain; null lookup never proves an outstanding open
  failed. Lost opens require a host restart; recover lost closes only by the exact retained view
  ID. Never adopt another tab's current view as evidence that an uncertain request settled.
  Prove ordering with deferred-response tests.
  Commit: `feat: own report drawer lifetimes`.
- **V17b2: Complete.** Shared-drawer report selector, embedded wrapper, loading/error/diagnostic states and
  explicit force confirmation. Activity renews only during visible inspection; dismissal or
  switching evidence releases the view. Match drawer mocks and add stories plus browser coverage.
  Commit: `feat: inspect tool reports in the run drawer`.
- **V17b3: Complete.** Browser lifecycle checks for dismissal during startup, replacement, reopening,
  hidden-tab expiry and failures with raw evidence still usable. Record checkpoint V-C, including
  any separately pending native Safari check. Commit: `test: verify report drawer lifecycle`.

**Checkpoint V-C: Automated checks complete.** [Evidence and native Safari limit](docs/plans/v03-reports-checkpoint.md). Full matrix and review with real fake-peer report processes, not static UI
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

V18 execution slices (review and full matrix per slice; rollback newest first):
- **V18a: Complete.** Refresh opt-in peer probes for the approved Rascal `9ee6bc8` and Squint `db5cf78`
  snapshots. Fail invalid explicit binaries/contracts; use observed shutdown, ordered list image
  inputs, zero-spend evidence and both declared renderer kinds. Retire the obsolete estimate-as-run
  assertion. Commit: `test: verify v0.3 peer runs and reports`.
- **V18b: Complete.** Real-peer readiness/default lookup and selected configuration/restart, plus bound
  estimates and capped dispatch using a scripted paid Rascal filler. Verify Squint's real-profile
  estimate control without credentials; its scripted profile declares free actions.
  Isolate configuration/cache directories without changing sibling checkouts. Record any live
  provider-only checks separately. Commit: `test: verify real peer setup and estimate controls`.
- **V18c: Complete.** [Checkpoint](docs/plans/v03-real-peer-checkpoint.md). Approved Squint `54b1f41`
  fixes its viewer CSP violation; strict Chromium/WebKit checks pass. Browser acceptance against
  both isolated binaries, including report frames, list
  references and saved evidence after editing/restart. Record the real-peer checkpoint and
  remaining manual/provider limits. Commit: `test: record real v0.3 browser acceptance`.

**V19 — Complete locally: bring the implemented UI into alignment with the mockups.** Final task of the v0.3
client. The coordinator authorized starting it while V18c awaited the Squint CSP fix; V18c subsequently passed its separate strict gate. Use `work/mockups.html`, `comparison-ui`, `drawer-ui`, `examples-ui`,
`journey-ui`, `wipe-ui`, `interface-study`, `activity-hierarchy` and `preview-controls` HTML studies.
Capture current screens and corresponding mock views, then record a short screenshot-backed gap
list under `docs/reviews/`: layout, spacing, typography, colour tokens, iconography,
empty/loading/error states and responsiveness. Fix the gaps within reason. Keep behavior required
by the contract, accessibility or meaningful tests when it differs from a mock, and document
each intentional divergence. Update Storybook stories alongside their components. Add Playwright
screenshot regressions for each corrected screen, with stable fixture data and deterministic
states; review baselines rather than accepting changes blindly. Run the full matrix and review
before local merge. Files depend on the gap list; split into named screen-sized tasks with checks
and docs per slice. Mark explicitly authorized this pass; no additional approval gate.
Depends: completed V01–V17; final acceptance also requires V18c. Commit strategy: `docs: record UI fidelity gaps`, then focused `style:`/`test:`
commits per screen. Rollback visual slices in reverse order while retaining V18's working protocol
and accessibility behavior. The coordinator reviews and pushes; this session does not push/tag.

V19 execution slices (review and full matrix per behavior slice; rollback newest first):
- **V19a: Complete.** [Audit](docs/reviews/2026-11-02-ui-fidelity.md). Screenshot-backed audit of current UI and mock counterparts, with named gaps and
  intentional contract/accessibility divergences. Commit: `docs: record UI fidelity gaps`.
- **V19b: Complete.** Workspace hierarchy: tool selection in the application header, contextual tool title,
  subordinate action picker, quiet library navigation and more space for results. Keep desktop
  input scrolling and narrow/short-window page scrolling. Shared shell/picker stories and
  deterministic desktop/mobile screenshots. Commit: `style: restore workspace tool hierarchy`.
- **V19c: Complete.** Shared drawer evidence navigation and report layout: compact, accessible navigation
  without a second row on desktop; narrow-screen overflow remains reachable. Cover inspector,
  pipeline and report states in stories and screenshots. Commit: `style: refine evidence drawers`.
- **V19 visual-runner correction: Complete; hosted run 41436171703 passed.** Normalize OS scrollbar geometry only during capture; use
  region baselines with a shared antialiasing tolerance, prove layout/colour mutations fail,
  and upload screenshot diffs. Covers V19b/c and is required for V19d.
  Commit: `test: stabilize visual regions across macOS runners`.
- **V19d: Complete locally.** Examples and preview polish: self-contained example styling, clear saved entries and
  empty/loading/error states; inspect comparison controls against the wipe/comparison mocks.
  Add screenshot coverage for the retained/adjusted previews, verify responsive states, and
  record remaining deliberate divergences. Full matrix and review passed; final hosted CI follows
  the coordinator push. Commit: `style: refine examples and preview states`.

### Approval, sequencing and rollback checkpoints

Mark approved V01–V18 on 2026-11-01 after the two review rounds and subsequently added V19
as the final UI fidelity pass. The initial proposal was
commit 0033af8. Implementation proceeds in order with the per-task checks below; the coordinator
reviews and pushes local merges. Paid dispatch was gated on the complete guard, now implemented
and verified by the checkpoints above.

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

## M5 — v0.4 operations and portable pipeline experiments

**Status: owner approved for implementation, 2026-11-02.** Goal 1
(V01–V19) is complete at `851b850`, with hosted CI run `41438647360` green. This milestone
adopts v0.4; it does not reopen the completed v0.3 client or authorize distribution work.

### Sources and completion boundary

Normative source: table-manners main `d1c49a7`, `CONTRACT.md` 2.3, 3–3.4, 4.5, 4.7–4.9,
5–5.2 and 10; `schemas/core/{description,result,session,refusal,invocation-bundle}.json`,
`schemas/operations/*`, and MAPPING's v0.4 table. Peer integration targets are Rascal
main at `eed6397` (M10 merged, exact v0.4 pin; m10 tag pending CI) and Squint main at
`6dd1e3a` (M10 merged, exact v0.4 pin). Read those revisions without
changing sibling worktrees. The coordinator must approve replacement revisions. Hub research
in `../wafflemaker/docs/v04-design.md` explains intent; the final contract takes precedence
where its earlier examples differ. The coordinator confirmed crates.io publication of table-manners
`0.4.0`, tagged `v0.4.0` at the reviewed `d1c49a7`. Use the published exact pin directly; no
temporary source dependency is needed. Peer release status still requires verification, not inference
from a branch name. These peer revisions supersede the earlier branch-head targets.

Completion means a person can run Line, inspect its Location, send it to Rasterize, save the
native Mask, and run Feather with changed parameters without rerunning a finder, filler or
any upstream operation. They can inspect the exact resolved invocation and export a portable
single run or selected chain for another directory and a fresh tool store. Squint remains a
task-based tool; no private operations are fabricated. Ordinary tasks, estimates, paid caps,
progress, cancellation, reports and saved evidence retain the v0.3 behavior.

### Host decisions and UI

- Cut over live dispatch to exactly contract `0.4.0`; retain MCP `2025-11-25` and the current
  owned-child/session machinery. No v0.3 adapter, private Rascal envelope, workspace-id lookup
  or action-name heuristic. Older saved evidence remains readable but cannot acquire invented
  invocations, artifacts or replay guarantees.
- Start from [activity hierarchy](work/activity-hierarchy.html) and
  [journey](work/journey-ui.html): compact tool selection in the header; Playground for tasks,
  Operations for independent pieces, Utilities only when declared. Use the same focused action
  picker in each, grouped by declared `group` with labels from `vocabulary.group`. Groups stay
  inside the searchable picker, never a growing category strip. Hide absent kinds rather than
  invent actions. Inspection, examples, setup and pipeline export use the existing shared drawer.
- Keep bounded desktop Input/Result panes, a visible Run footer and normal page flow on narrow
  screens. Input ports precede ordinary parameters. An image port accepts an owned image or
  saved image artifact; a location port accepts a shared location value or artifact; native
  ports select typed artifacts. Scalar/list/optional states come from metadata. Constants and
  choices remain schema-driven parameters, not tool-specific controls.
- Result shows one card per declared output occurrence: port, type, preview when supplied,
  origin run, preservation state, Inspect and Send to. A Location can use its tool preview or
  the shared location display; a native record remains opaque JSON. No fabricated image,
  forced before/after comparison, or PNG standing in for a Mask. Salvaged outputs on failed or
  cancelled runs remain usable when the contract supplies them.
- Send to fills a new draft and records the source attempt/output occurrence. It never runs
  the receiver, imports silently, or reruns provenance. Filter suggestions by declared type,
  version and known `on` file digest; unknown `on` is not a compatibility guarantee. The tool
  decides frame compatibility. Native types stay with their named tool; shared image/location
  transfers still require the receiving tool to accept or import the input.
- Store each result's `invocation` immutably beside original wire evidence. Require it on every
  result, permitting null only for an invalid run whose arguments did not resolve. Store typed
  artifact references and their source occurrence separately from previews and opaque records.
  Keep frame and identity opaque. Never recover native artifacts by searching tool directories.
- Preserve portable non-image artifact inputs/outputs through the tool's export control, deduplicated by
  artifact and export-byte identities. Preservation is bounded background work, separate from
  run outcome: failure leaves a visible workspace dependency and explicit retry. Save Example
  and bundle export must report incomplete preservation, never claim portability prematurely.
  Portable images remain original owned bytes; `portable: none` stays workspace-dependent.
- Every non-image artifact reference follows the same preservation, export/import and chain-edge
  rules, including shared `location` with portable:json as well as namespaced tool-native types.
  A location passed by value stays in invocation.arguments as a value: it has no replay edge, even
  if the user copied it from an earlier result. Image references use original owned image bytes.
- An invocation bundle is an explicit export, not an automatic archive of every run. Save its
  immutable manifest, digest, member references and selected-attempt links in the local library.
  Keep it distinct from the existing gzip evidence archive. Included input bytes are the default;
  omissions are explicit and labelled. Requirements show names only, never credential values.
  Download UI identifies that the original result may contain local paths and tool-specific data.
- Replace registration `roots` with separate `allow_dirs` (child input permission) and
  `evidence_dirs` (host reading returned images/run directories). Reject old registration fields
  with documented replacement instructions; do not silently migrate or broaden grants. Pass
  canonical `--allow-dir` arguments for the dedicated materialized-input/import area plus explicit
  registered input directories, not the whole library/database or home. Remove roots capability
  and roots/list entirely. Read effective session `allowed_dirs`; config may narrow the request.
  Changing grants requires an explicit new session and invalidates prepared estimates.
- New artifact exports must resolve inside explicitly registered `evidence_dirs`, even though the
  contract permits a tool to choose an export area outside run.dir. An out-of-bound export is refused
  with setup guidance; never infer an export directory from a tool name. Read only the exact returned
  regular file with race-safe opening, bounded size and export hash checks. Existing v0.3 document
  rendering keeps its approved exact-file/isolation policy; this task does not broaden that policy. No export path is executed, exposed
  as a browser path, or used to expand `allow_dirs`/`evidence_dirs`. Do not delete tool-owned files.

### Task order and acceptance

Each unchecked item below is implementation work, not a claim that a peer or host feature has
shipped. File lists name primary areas, not exemptions from tests, stories or documentation.
Budget each implementation commit to at most five authored files; generated assets/snapshots
are reported separately. If wiring exceeds that budget, add named `.a/.b` sub-tasks with their
own checks **before** building. Every code slice uses failing behavioral tests first, targeted
verification, the full matrix and review before local merge. Listed commit subjects describe the parent
intent; named sub-slices use scope-specific subjects and types. No unnumbered tooling follow-ups.

#### M5.01 — Establish an independent v0.4 peer

- [x] Add schema-validated core/operations fixtures from the pinned contract, plus independently
  authored fake actions with unrelated names, scalar/many ports, task outputs and all three kinds.
- [x] Fake scenarios cover preview/no-preview, portable/opaque types, indexed binding refusal,
  invocation on all outcomes, artifact resources and export/import controls without provider calls.
- [x] Keep fake behavior independent of real peer implementation and host types; document each
  fixture's source. Contract semantics have assertions beyond schema validity.

**Depends:** owner approval. **Areas:** `testkit/`, contract fixture tests, Cargo manifest/lock.
**Verify:** positive/negative fixture suite and fake wire tests. **Commit:** `test: add independent v0.4 operations peer`.

**Planned build slices** (ordered, each with its own targeted checks and matching docs):

- **M5.01a: Complete — schema fixtures and negative cases** — acceptance 1 and schema portion of 3; fixture tests.
- **M5.01b: Complete — independent fake scenarios** — acceptance 2 and wire independence in 3; fake protocol tests.

M5.01b is split before implementation to keep the wire peer reviewable:

- **M5.01b1: Complete — Discovery and session probe:** independent stdio initialization/describe/session/runs,
  canonical `--allow-dir`, no roots request, fresh session on restart, bounded input and EOF shutdown.
  Verify renamed tool identity and invalid-directory exits using a line-delimited client. Commit:
  `test: add the v0.4 discovery wire peer`. This remains a control-only probe until b3.
- **M5.01b2: Complete — Artifact controls:** artifact/preview resources and verified idempotent export/import
  of native and shared-location bytes, including structured refusals. Controls create no runs/spend.
  Verify restart, corrupt bytes, missing identity and refused paths. Commit:
  `test: add v0.4 artifact control scenarios`.
- **M5.01b3: Complete — Operation execution:** scalar/list binding, independent-piece runs, resolved invocations,
  progress/cancellation and salvaged outputs, using actual request data and no providers. Verify a
  chain, indexed pre-run refusals with released IDs, and controls during a held run. Commit:
  `test: execute independent v0.4 operations over stdio`.
M5.01b2 is split before implementation into **b2a (portable store)** and **b2b (wire controls)**.
The first validates and persists the fake's own portable records, serves stored identity/preview
bytes, and tests idempotence, corruption, relocation and grants. The second connects export/import
and resource reads to stdio, with wire schema/refusal/no-run checks. This separates storage
behavior from protocol dispatch while keeping each commit within five authored files.
M5.01b3 is split before implementation into **b3a (binding)**, **b3b (resolved results)**,
and **b3c (wire execution/lifetime)**. b3a checks declared scalar/list ports, all indexed binding
failures, defaults and shared image/location inputs, extending the synthetic store for its owned
image fixture. b3b executes the fake steps from resolved inputs and builds matching invocation,
artifact and image/location profiles for every outcome. b3c adds run allocation/claim release,
progress, held-run controls, cancellation/salvage and EOF ownership over stdio. The fake exercises
its own documented synthetic image/frame, not real image-repair algorithms: unsupported input
bytes/frames fail explicitly rather than silently substituting an example. Each slice owns its
behavioral tests and stays within five authored files; split b3c again if its wiring exceeds that.
M5.01b3c is split before implementation into **c1 (run records and claims)**,
**c2 (callable schemas and wire execution)** and **c3 (held-run cancellation and EOF ownership)**.
c1 persists allocation references before work, pages runs by position, exposes journals/final
results, atomically claims correlation IDs and releases only unallocated refused calls. Direct
concurrency/restart/paging tests precede wiring. c2 exposes the action schemas and executes calls
through those records, using the stdio default of path previews. c3 keeps controls responsive
while a run is held, journals progress/heartbeats, and proves cancel-before/after-allocation and
EOF cleanup. Each part stays within five authored files; its docs distinguish direct state tests
from completed wire behavior.
M5.01b3c2 is split into **c2a (standalone callable schemas)** and **c2b (wire dispatch)**.
c2a shares the resolver's declared input schema with future tools/list, expanding only pinned
contract references without changing sibling validation semantics. It tests scalar/list inputs,
literal/shared references and closed schemas without external resolution. c2b connects those
schemas and completed run records to stdio. This avoids duplicating call validation in the peer
and keeps each slice within five authored files.
Each slice includes testkit documentation, targeted tests, the full matrix and review.

Review split **M5.01b3c2b** into **c2b1 (terminal outcome finalization)** and **c2b2 (wire dispatch)**
before further implementation. c2b1 exposes a resolved, not-yet-executed result for I/O failures
before work, and ensures a failed terminal journal append cannot prevent an otherwise writable
run.json from recording the outcome. Preserve completed artifacts and cancellation status; return
the exact saved result, including any journal diagnostic. A native artifact already written
also survives failed preview delivery, using its resource preview as the fallback. Test missing
journals, retained artifacts and immutable final evidence directly. c2b2 then builds optional image blocks before finalization and converts
post-allocation recoverable I/O failures into results; only an unwritable final store remains a
server-level failure. Both slices retain the five-file bound and full verification/review.

**M5.01b3c2b2 implementation scope:** expose every declared callable, bind the actual arguments,
claim/release correlation IDs through ownership handles, execute bounded synchronous steps,
and persist/read results and journals with position-based paging. Use path previews by default
on stdio and opt-in image content blocks. Invalid inputs and I/O outcomes are saved tool results;
pre-run binding/schema refusals are RPC errors. Verify a renamed wire chain, ID retry/reuse,
paging, original-byte evidence and both delivery modes. Held workers, progress and cancellation
remain c3 and are explicitly absent from this intermediate peer.

M5.01b3c3 is split into **c3a (cancellable dispatch boundaries)** and **c3b (worker lifetime and wire notifications)**.
c3a separates acceptance/claiming from allocation, adds explicit cancellation at allocation and
finalization, and exposes allocation/execution observations for the upcoming worker. Direct tests
prove cancelled queued IDs remain claimed without runs, allocated cancellation retains evidence,
and finalization cannot be changed by a late cancel. c3b keeps one execution worker with responsive
control requests, monotonically increasing progress/heartbeats carrying structured run references,
a file-released held-run fixture, cancellation without an MCP reply, and EOF cancellation/join.
If worker wiring exceeds five authored files, split the worker core and its stdio integration
before implementation; every slice retains targeted tests, review and the full matrix.

M5.01b3c3b is split into **c3b1 (owned worker core)** and **c3b2 (stdio integration)**.
c3b1 adds a single owned execution worker with bounded admission, cancellable queued requests,
file-released holds after execution, structured monotonic progress/heartbeats and shutdown join.
Direct channel tests verify responsiveness, ID ownership, cancellation/salvage and EOF-equivalent
shutdown without depending on browser timers. Its module is test-only until c3b2 connects it to
stdio; c3b2 then tests actual protocol notifications, controls during holds, no cancelled replies
and stdin EOF ownership. Both remain within five authored files and run the full matrix/review. The c3b2 code commit
updates the testkit README alongside the five integration files; a following documentation-only
checkpoint records M5.01 acceptance in this plan before the coherent slice is merged.

**M5.01a implementation checkpoint (2026-11-02, complete):**
`testkit/v04_fixtures.rs` independently authors the descriptions, result outcomes, artifact
resources and binding/export/import/session examples. It imports neither host nor peer code.
The synthetic Token/Memo types, action names and resource identities belong only to the test kit;
source/image-artifact fingerprints come from the owned `ui/tests/fixtures/sample.png`. Location
and Token previews encode actual deterministic PNG bytes, distinct from native records. Fixture
input identities/timestamps are deterministic examples, not claims of live execution.
`tests/v04_contract_fixtures.rs` validates them against published v0.4 schemas and checks semantic
relationships that schemas alone cannot establish. Both the missing description and absent
fixture catalogue failed before implementation. Review regressions then failed for the import
grant, unchanged output identity and absent task/many-output cases before their fixes. Ten targeted
tests now pass, including declaration/result/argument/profile consistency checks. All 21 published
schema files match `d1c49a7` byte for byte. The dev-only `table-manners-v04 = =0.4.0` alias allows
this independent test work while live dispatch still uses exactly v0.3; M5.03 replaces the live
pin and removes the temporary alias, with no dual-version production dispatch. M5.01b still
owns stdio behavior and the wire checks; these static examples do not claim execution coverage.
Image-port examples supplied by artifact reference retain that reference in their invocation;
the task's image input records `{expect}`. This also matches the approved Rascal revision's
operation invocation handling, without importing its implementation into the fake.
Full matrix passed under the shared lock: fmt, clippy, 253 Rust tests (4 opt-in ignored),
typecheck, 53 UI unit tests, 2 asset-tool tests, Storybook, reproducible assets/check-ui,
release build and 220 Chromium/WebKit tests (8 opt-in skipped). Lockfile and prebuilt assets
were unchanged by the matrix. Independent review approved after the fixture corrections above;
hosted CI remains separate coordinator-owned evidence after push.


**M5.01b1 implementation checkpoint (2026-11-02, complete):**
The independent `v04-operations` stdio probe handles discovery/session/empty runs, explicit
canonical grants (invalid grants exit 64), renamed identities, fresh sessions and EOF shutdown.
It never requests roots or writes a run. Only describe is callable until the next slices add
controls and execution. Five wire tests pass after failing on the legacy roots request and
invalid-directory exit. Review strengthened the input bound test: valid pings at 1 MiB pass,
while 1 MiB + 1 fails with stdin still open. Deliberately raising the limit makes that test fail.
Full matrix passed under the shared lock: fmt/clippy, 258 Rust tests (4 opt-in ignored),
typecheck, 53 UI tests, 2 asset-tool tests, Storybook, reproducible assets, release build,
and 220 Chromium/WebKit tests (8 opt-in skipped). An unrelated preview occupied port 4173;
a temporary config changed only the local test server/base URL to 4187 and was removed after
verification. The default CI configuration is unchanged. Independent review approved; no
production behavior, dependency lock or prebuilt asset changed.


**M5.01b3b implementation scope:** build schema-valid resolved results for all synthetic actions,
including ordered many outputs, shared-profile mirrors, preview locators and null invocation only
when invalid inputs do not resolve. Accept cancellation between units of local work and retain
completed outputs on failure/cancellation. Keep run persistence/progress/timing in b3c; direct
executor tests prove result construction without claiming a live wire execution path.

**M5.01b3a implementation scope:** resolve inputs without run allocation or store writes.
Use published schemas for whole-call shape and report all typed-reference failures in port/list
order. Preserve distinct image-reference, image-locator and literal-location invocation forms;
apply defaults and remove only delivery options. Reject mismatched original bytes or synthetic
frames explicitly. Optional pixel hashes cannot override matching file identities. Test imported
native references after deleting the producer workspace. This groundwork does not dispatch runs.

**M5.01b2b implementation scope:** expose store controls and artifact/preview resource reads
through the independently framed peer. Published control schemas are self-contained in tools/list.
The transport envelope rises to 2 MiB to accommodate 1 MiB inline files after base64; the decoded
file limit does not change. Wire tests must verify error-vs-result envelopes, no run/spend changes,
fresh session with persistent artifacts, and valid inline requests on both sides of the limit.

**M5.01b2a implementation checkpoint (2026-11-02, complete):**
The independent portable store accepts complete verified synthetic Token/location records,
retains identities across restart and workspace moves, and serves real fingerprinted preview
bytes. Import verifies optional digests, media/type/record consistency and directory grants
before recording anything; native Memo remains nonportable. Stable exports live outside runs.
Eight direct store tests pass. Review regressions first reproduced existence-disclosing path
errors, a concurrent duplicate-import race, invalid grant acceptance and the wrong error class
for corrupt stored entries. Fixes make all path failures indistinguishable, verify the winning
record after duplicate persistence, validate grants at the store boundary and preserve corrupt
entries while reporting an internal error. No host types or peer implementation are imported.
The full matrix and review passed: fmt/clippy, 266 Rust tests (4 ignored), typecheck, 53 UI
unit tests, 2 asset-tool tests, Storybook, reproducible assets, release build and 220 browser
tests (8 skipped). Browser verification again used the temporary 4187 override because an
unrelated project held 4173. Lockfile/prebuilt assets are unchanged. M5.01b2b owns stdio wiring;
these controls have not yet been exposed through the probe.


**M5.01b2b implementation checkpoint (2026-11-02, complete):**
The peer now serves reserved import/export controls and artifact/preview resources over stdio,
with self-contained published input schemas. Successful controls return MCP structured content;
refusals are JSON-RPC errors without a run. Seven wire tests pass after missing-control and
missing-refusal failures, and an MCP object-schema assertion failed before its correction.
Valid padded files at 1 MiB import successfully through base64; 1 MiB + 1 is refused without
stopping the peer. The 2 MiB wire envelope is separately bounded. No control changes the session
ledger or run list, and stored artifacts reopen in a fresh session. The full matrix passed:
fmt/clippy, 268 Rust tests (4 ignored), typecheck, 53 UI unit tests, 2 asset-tool tests,
Storybook, reproducible assets, release build and 220 browser tests (8 skipped). Browser
verification used the same temporary 4187 override; lockfile/prebuilt assets are unchanged.
Independent review approved. Schema expansion is intentionally limited to the two pinned
acyclic control schemas; do not reuse it for recursive or unevaluated-property schemas.
The peer remains control-only until b3. Tests of schema-invalid calls in later client work
must accept plain invalid-params errors, not depend on the fake store's richer refusal data.


**M5.01b3a implementation checkpoint (2026-11-02, complete):**
The independent resolver validates declared call shape, binds scalar/list ports in order,
collects indexed reference failures, and distinguishes pre-run refusals from invalid image/location
inputs. Defaults are resolved; delivery flags stay out of replay arguments. It reads actual
owned-fixture bytes under explicit grants and preserves artifact-reference vs literal-location
forms. Binding allocates no run or artifact. Nine binding tests pass alongside the store tests;
portable inputs bind after deleting their producer workspace. A pixel-hint regression failed
before ignoring optional pixel hashes where the original file identity matches. Review tightened
image-reference remedies and added negative marker/media/path and binding-priority coverage;
the actionable-remedy assertion failed first. Both review rounds approved. The full matrix
passed: fmt/clippy, 277 Rust tests (4 ignored), typecheck, 53 UI tests, 2 asset-tool tests,
Storybook, reproducible assets, release build and 220 browser tests (8 skipped), with the same
temporary local 4187 override and unchanged lockfile/prebuilt assets. Results and execution
remain b3b/b3c work; no production host behavior changes in this slice.

**M5.01b3b implementation checkpoint (2026-11-02, complete):**
The independent result builder executes all synthetic steps from resolved inputs, preserving
ordered ports and mirroring artifact previews into the shared profiles. Ten executor tests cover
chaining, native parameter changes, all preview locators and original-byte fingerprints, early
and partial cancellation, partial output failure, invalid inputs and final timing. Regression
tests failed first for missing execution/cancellation support, filesystem errors mislabeled as
internal, and absent timing finalization. File failures now report `io` with plain reasons;
updating elapsed time preserves terminal outcomes and derives finish from start plus elapsed.
Delivery flags never change input identity, including unresolved invalid inputs. Independent
review approved the corrected slice. The full matrix passed: fmt/clippy, 287 Rust tests (4 ignored),
typecheck, 53 UI tests, 2 asset-tool tests, Storybook, reproducible assets, release build and
220 Chromium/WebKit tests (8 skipped). The temporary local 4187 override was removed; lockfile
and prebuilt assets are unchanged. Wire allocation, claims, progress and persistence remain b3c.

**M5.01b3c1 implementation checkpoint (2026-11-02, complete):**
The independent run store persists allocation references/action and readable journals before
work, then writes immutable terminal results last. Two-entry pages use numeric start time plus
ID, including equivalent RFC3339 spellings. Session correlation claims are atomic and return
private ownership handles: refused duplicates and stale owners cannot release or allocate a
current claim. Restart preserves run evidence but resets claims; relocation changes only the
read response's local `run.dir`, leaving stored evidence and identity unchanged. Eight direct
tests cover contention, refusal release, failed allocation, paging, unpublished records,
immutable results and reference integrity. Missing implementation and forged reference tests
failed first. Review found claim-ownership and invalid-fixture gaps; mutations removing ownership
and publication guards failed their corrected regressions. Final review approved. The full
matrix passed: fmt/clippy, 295 Rust tests (4 ignored), typecheck, 53 UI tests, 2 asset-tool tests,
Storybook, reproducible assets, release build and 220 browser tests (8 skipped). Local browser
checks used the temporary 4187 override; lockfile/prebuilt assets are unchanged. Wire dispatch,
progress and cancellation remain c2/c3. The fake does not claim crash recovery; a failed terminal
publication may repeat its private journal event, and only `run.json` establishes completion.

**CI follow-up — report close completion (2026-11-02, complete):**
Hosted run 41507896035 exposed a pre-existing WebKit assertion deadline racing the host's two
close-grace stages. The force-required lifecycle test now waits for the actual close response
and retains its stale-observation/remount assertions. All twenty WebKit repetitions passed.
Independent review and the full matrix passed: fmt/clippy, 295 Rust tests (4 ignored), typecheck,
53 UI tests, 2 asset-tool tests, Storybook, reproducible assets, release build and 220 browser
tests (8 skipped). No production behavior, retry count or assertion timeout changed.
Evidence and disposition: [report close review](docs/reviews/report-close-deadline.md).
Hosted rerun 41511603508 passed for the permanent fix; the schema/wire merge gate is clear.

**M5.01b3c2a implementation checkpoint (2026-11-02, complete):**
The fake's resolver and upcoming callable catalogue share one declared-input schema builder.
Pinned input references expand locally; reference siblings remain at the same level so closed
objects and `unevaluatedProperties` keep their meaning. Four new tests cover all six actions,
scalar/list forms, literal/shared references, delivery options, closed objects and published
control inputs without external resolution. The missing-module test failed first; all binding,
result and run-record regressions pass after extraction. Review approved within the documented
owned, acyclic fixture scope. Clippy caught an unused extraction variable, which was removed.
The final full matrix passed: fmt/clippy, 299 Rust tests (4 ignored), typecheck, 53 UI tests,
2 asset-tool tests, Storybook, reproducible assets, release build and 220 browser tests
(8 skipped). Local browser checks used the temporary 4187 override; lockfile/prebuilt assets
are unchanged. The wire peer remains control-only until c2b.

**M5.01b3c2b1 implementation checkpoint (2026-11-02, complete):**
Resolved invocation evidence can be constructed without executing work. Terminal journal failures
no longer suppress an otherwise writable final result: completed runs become I/O errors, while
prior failure/cancellation outcomes retain their status and gain a diagnostic. The exact saved
value is returned for dispatch. Completed native outputs also survive failed preview delivery,
with their canonical resource preview retained; that address does not guarantee a later read.
Missing journals remain unavailable resources even when final evidence is readable. An unwritable
final result remains a server storage failure and retains the claimed run reference.
Regression tests failed first for absent resolved evidence and dropped completed outputs. Review
corrected a final-store test that stopped before the write; its permission fault now proves the
journal append happened, the final write failed, and retry preserves the claim and publishes the
exact result. Follow-up review found no blockers. The full matrix passed: fmt/clippy, 303 Rust tests
(4 ignored), typecheck, 53 UI tests, 2 asset-tool tests, Storybook, reproducible assets, release
build and 220 browser tests (8 skipped). The temporary local 4187 override is removed; lockfiles
and prebuilt assets are unchanged. Wire dispatch remains c2b2, then held-run cancellation is c3.

**M5.01b3c2b2 implementation checkpoint (2026-11-02, complete):**
All six independent actions are callable over stdio through their standalone schemas. A renamed
Locate → Stamp → Soften chain verifies native identity, effective arguments, preview-byte hashes,
saved results, journals and two-entry paging. Binding/shape refusals release their owned ID;
completed and failed runs retain it. Stdio uses path previews by default and sends image content
only on request. Invalid inputs and I/O failures return saved tool results with matching text,
structured content and resource evidence. Two deterministic dispatch faults failed first: missing
execution journal and missing preview storage before content delivery. Both now finalize an I/O
result while retaining resolved invocation and completed outputs; final-record validation/write
failure remains a server error with the claim retained. Review approved. The full matrix passed:
fmt/clippy, 309 Rust tests (4 ignored), typecheck, 53 UI tests, 2 asset-tool tests, Storybook,
reproducible assets, release build and 220 browser tests (8 skipped). Temporary port 4187 config
was removed; lockfiles/prebuilt assets are unchanged. Execution remains synchronous until c3;
progress, held-run controls, cancellation and EOF worker ownership are not yet implemented.

**M5.01b3c3a implementation checkpoint (2026-11-02, complete):**
Dispatch acceptance claims/validates without allocating, then executes through a per-request
cancellation guard. The guard serializes cancellation with allocation and terminal publication.
Queued cancellation retains its ID with no run; allocated cancellation after invocation resolution
retains invocation and completed outputs, saves cancelled and returns no reply. An unresolved input
rejection remains invalid with null invocation and no reply, as clarified and tested in c3b2.
Publication is the cutoff: an earlier resolved execution failure remains in notes if cancellation
arrives before publication, while a later cancel cannot rewrite saved evidence. Even a failed final save produces no cancelled-request
reply; it logs a stderr diagnostic and retains the claim. Seven direct dispatch tests cover these
boundaries and previous storage faults; the missing API and cancelled-save error tests failed
first. Two review rounds found no remaining blockers after clarifying the cutoff and moving
reply suppression into the guard. The final full matrix passed: fmt/clippy, 314 Rust tests
(4 ignored), typecheck, 53 UI tests, 2 asset-tool tests, Storybook, reproducible assets, release
build and 220 browser tests (8 skipped). Temporary port 4187 config is removed; lockfiles and
prebuilt assets are unchanged. The wire path is still synchronous; c3b owns worker/notification
integration. Production remains on v0.3 until M5.03.

**M5.01b3c3b1 implementation checkpoint (2026-11-02, complete):**
The owned serial worker admits at most 32 active/queued requests and leaves controls independent.
Per-request progress keeps an increasing elapsed counter across queue/allocation, carries no run
reference before allocation, and includes the four-field reference afterward. The explicit
file-released hold drives running/queued heartbeats; cancellation stops further progress and
shutdown cancels accepted work then joins. Six channel-based lifecycle tests cover queued ID
ownership, cancellation without replies, saved output salvage, admission refusals, output failure,
and response-ID reuse. A blocked response callback reproduced removal-after-delivery ordering;
entries now leave the active registry before delivery. A queued-heartbeat test failed first,
and disabling cancellation failed the corrected drain-based regression, proving shutdown cannot
mask a missed cancel. All twenty repetitions passed. Two review rounds found no remaining
blockers. The full matrix passed: fmt/clippy, 320 Rust tests (4 ignored), typecheck, 53 UI tests,
2 asset-tool tests, Storybook, reproducible assets, release build and 220 browser tests (8 skipped).
Temporary port 4187 config is removed; lockfiles/prebuilt assets are unchanged. This worker module
remains test-only until c3b2 connects the stdio reader, notifications and EOF lifecycle.

**M5.01b3c3b2 implementation checkpoint (2026-11-02, complete):**
The stdio reader now owns the worker and serializes complete output messages while continuing to
serve describe, session, artifact controls and resources during execution. Cancellation notifications
use the accepted request ID; stdin EOF and malformed-input exits cancel and join owned work.
Six additional child-process tests cover responsive controls, queued progress without a run,
queued/allocated cancellation without replies, EOF/error cleanup, and cancellation after rejection.
The last regression failed first with a cancelled/null-invocation result. Per the coordinator's
ruling, a rejection before invocation resolution now remains invalid with its input error and null
invocation; the cancelled request receives no MCP reply. Cancellation becomes the saved outcome
only after invocation resolves. Existing resolved-failure diagnostics and salvaged outputs remain.
All twenty repetitions of the six lifecycle cases passed, and both review rounds found no blockers.
The final full matrix passed: fmt/clippy, 326 Rust tests (4 ignored), typecheck, 53 UI tests,
2 asset-tool tests, Storybook, reproducible assets, release build and 220 browser tests (8 skipped).
Temporary port 4187 config is removed; lockfiles and prebuilt assets are unchanged. M5.01 is complete:
the independent peer exercises v0.4 operations, controls and lifecycle without host types, real-peer
code or paid calls. Production adoption remains in M5.03 after the M5.02 directory-grant cutover.

#### V20 — Image import feedback (M5-adjacent; coordinator-approved 2026-11-02)

This follow-up does not change M5's task order or block M5.01b. The demo showed that a
15000×7500 image passes the library bound but can exceed a tool's own bound. Original bytes,
dimensions and fingerprints remain authoritative; thumbnails never replace run inputs.

- [ ] Read PNG/JPEG header dimensions without a full decode and immediately show dimensions
  and estimated decoded memory, with explicit import progress and failure feedback.
- [ ] Generate bounded thumbnails from a downscaled decode; stop fetching/decoding the full
  original merely to display a thumbnail. Preserve full validation before an asset is ready.
- [ ] Display size refusals and any remedy supplied by the tool clearly. Do not guess a tool's
  limit or parse its private fields. The coordinator owns an additive next-contract proposal
  for optional `max_pixels` (and potentially width/height) on image slots and ports.
- [ ] Benchmark release-build import and preview stages with owned representative images,
  recording dimensions, byte size, timing, memory where measurable, and test hardware/build.
- [ ] Consider an explicit labelled "Make a resized copy" only if cheap and user-selected;
  never automatically resize fidelity inputs or overwrite the original.

**Build slices:** V20a measure/header/progress; V20b bounded preview pipeline; optional V20c
explicit copy. Each needs failing behavioral tests, a five-file implementation budget split
as needed, stories/browser evidence, full matrix and review. Header feedback is provisional;
malformed/truncated images must still fail validation before becoming usable. Codec support
and memory behavior must be measured before choosing a downscaled-decoding implementation.


#### M5.02 — Separate input grants from evidence access

- [x] Replace registration roots with explicit input/evidence directory fields, validate existing
  absolute canonical directories and argv ownership, and reject conflicting hand-written grant flags.
  Remove Rascal-only grant injection from development descriptors in favor of the shared launcher.
- [x] Child argv contains repeated `--allow-dir`; input/import staging is the only automatic grant.
  Preserve evidence/report read restrictions separately. No roots capability or roots/list remains.
- [ ] Setup shows requested and effective `allowed_dirs`; no-grant, narrowed config, symlink escape,
  invalid directory, restart and unchanged old-session spend have explicit tests and migration docs.
  Before changing real registrations, save their previous form; rollback restores that form only with
  the matching old binary/peers, never by translating live grants silently.

**Depends:** M5.01. **Areas:** registration/client/session, evidence/render authorization, dev descriptors.
**Verify:** argv capture and path-boundary integration tests, including an unrelated tool name.
**Commit:** `feat: replace MCP roots with explicit input grants`.

**Planned build slices** (ordered, each with its own targeted checks and matching docs):

- **M5.02a: Complete — registration, argv and read-boundary cutover** — acceptance 1–2 and registration rollback;
  path/argv tests.
- **M5.02b: Complete — staged grant setup display** — requested/evidence observations and effective-access states;
  live v0.4 narrowing/restart acceptance finishes in M5.03 as approved.

M5.02b is staged in three commits: **b1 (requested/evidence grant observation and boundary tests)**,
**b2 (grant display, stories and browser states)**, then **b3 (setup wiring and live refresh checks)**.
Each code commit stays within five authored files; generated prebuilt assets are a separate commit.
Run the full matrix and independent review on the complete joined slice before merging.
The b1 observation boundary is implemented: snapshot data comes from the serving client's validated
registration, keeps requested/evidence lists separate, and includes no guessed effective grant.
The targeted Rust boundary test and two UI parser tests failed first and now pass; final matrix/review
cover all three joined commits. b2's browser tests failed first before the display/stories existed.
The coordinator approved staging the display before M5.03: current v0.3 sessions cannot report
`allowed_dirs`, so live effective access must say Not reported, never copy the requested set or
pretend it is empty. Reported/narrowed/empty/loading/error states use independent stories now.
M5.03 must finish live effective-grant validation, config narrowing and session/restart checks
before M5.02's final acceptance is checked. No v0.4 fields are accepted through the v0.3 validator.
The b2 display distinguishes reported-empty, unreported, loading and unavailable access. Chromium
and WebKit checks cover narrow grants, separate result folders, escaped literal names and responsive
wrapping at 320/768/1024/1440px. b3's live setup regression failed before wiring; its refresh path
must clear effective status on failure while retaining the distinct registered folder observations.

Independent review approved the staged display and identified a directory-name serialization edge:
canonicalization can reach a non-UTF-8 target on Linux. Registration now rejects non-UTF-8 grants
before and after canonicalization instead of losing bytes or panicking during snapshot serialization.
The explicit-refusal regression failed first on macOS; the symlink regression is Linux-only because
macOS refuses to create such names. Linux execution remains unverified on this macOS-first checkpoint.

M5.02a is one coordinated registration cutover. Replacing the public descriptor fields requires
updating every Rust constructor and disposable JSON registration together; retaining an alias would
violate the no-shim decision. This slice therefore exceeds the five-authored-file limit for the
mechanical caller updates, alongside the registration/client/read-boundary changes and regression
tests. It stays on one branch until the full matrix and independent review pass. Production schema
validation remains v0.3 until M5.03, while the independent old-flow fake receives grants through argv.
No real local registration is migrated by this change; the documented backup/explicit replacement
procedure applies before using a new binary with an existing registration. Roll back the entire
slice together, never just the descriptor or the evidence-reader half.

**M5.02a implementation checkpoint (2026-11-02, complete):**
Registration now requires separate canonical `allow_dirs` and `evidence_dirs`, rejects old fields
with backup/re-registration guidance, and refuses hand-written grant flags. The shared launcher
passes only input directories and the coordinator's dedicated execution staging area. Evidence
capture and run/report resolution use only the evidence list; the document renderer's exact-file
boundary is unchanged. No MCP roots capability is advertised, and unsolicited roots/list returns
method-not-found rather than rmcp's successful empty default. Development descriptors and real-peer
smoke registrations rely on host staging, removing Rascal-only argument injection.
Seven directory-grant tests cover literal argv, missing/invalid/legacy fields, independent permissions,
the exact automatic grant, existing staging-symlink refusal and unsolicited roots requests. A render
regression checks that input permission cannot authorize reports. The original field tests and both
review regressions failed first. The full matrix also caught a test wrapper discarding appended
argv; forwarding its arguments restores real grant checks across restart. Two independent review
rounds found no remaining blockers. Final verification passed: fmt/clippy, 334 Rust tests (4 ignored),
typecheck, 53 UI tests, 2 asset-tool tests, Storybook, reproducible assets, release build and 220 browser
tests (8 skipped). Temporary port 4187 config is removed; lockfiles/prebuilt assets are unchanged.
Real registrations were not touched. The old V18 Squint snapshot needs roots and is explicitly
unsupported on this intermediate build; M5.03 adopts the approved v0.4 peer and validator. M5.02b
still owns the requested/effective grant display and configuration-narrowing/session checks.

**M5.02b implementation checkpoint (2026-11-02, staged display complete):**
Tool setup now separates the serving child's requested input grants from Wafflery's result-read
permissions. Live v0.3 sessions show Not reported for effective access; refresh shows loading or
Unavailable without replacing unknown access with an empty or requested set. Independent stories
cover reported, narrowed and empty grants alongside long literal paths. Chromium/WebKit tests
check separation, refresh failure/recovery, escaping and 320/768/1024/1440px layouts; screenshots
were inspected for the narrow component and live setup. The requested/evidence snapshot boundary,
parser and live wiring tests failed first. All three implementation slices stayed within five
authored files; generated assets were committed separately. Independent review and follow-up found
no remaining blockers; the directory-name refusal above addresses its serialization finding.
Final verification passed under the shared heavy-check lock: fmt/clippy, 335 Rust tests (4 ignored),
typecheck, 55 UI tests, 2 asset-tool tests, Storybook, byte-identical clean UI builds, release build
and 226 browser tests (8 skipped). Temporary port 4187 config was removed and lockfiles/prebuilt
assets remained unchanged. M5.02's final acceptance stays open until M5.03 validates and displays
live v0.4 effective grants and verifies narrowing/restart with retained old-session spending.

#### M5.03 — Cut over live validation to v0.4

- [ ] Adopt published `table-manners = "=0.4.0"` directly and update the lockfile. Verify the packaged
  schemas/fixtures against reviewed tag `v0.4.0` at `d1c49a7`; report any packaging discrepancy before
  cutover. No path, git or temporary source pin is needed in development or CI.
- [ ] Validate kinds/groups/ports/artifact types, result invocation, session grants and new refusals.
  Reject inconsistent metadata and schema collisions before presenting an executable action. Reserved describe/estimate/export/import
  controls cannot appear as runnable actions.
- [ ] Existing task-only fake flows, paid preparation and current-contract report viewers still
  work. Old saved evidence remains readable/exportable; launching a renderer is a live operation
  and requires v0.4 saved declarations/results. After checkpoint A, only v0.4 live peers are supported. Point opt-in real-peer harnesses and
  development instructions at the approved M10 revisions with explicit version preflight; incompatible
  installed peers explain the required update, never pass a smoke check by silently skipping it.
  M5.21 owns full peer verification. Test v0.3 rejection rather than adding fallback dispatch.

**Depends:** M5.01–02. **Areas:** contract validation, description/session consumers, testkit defaults,
published dependency wiring. **Verify:** contract mismatch tests and complete existing suite.
**Commit:** `feat: adopt the v0.4 client contract`.

**Planned build slices** (ordered, each with its own targeted checks and matching docs):

- **M5.03a: v0.4 semantic validators** — acceptance 2; malformed-contract tests.
- **M5.03b: live cutover and reproducible test/dependency wiring** — acceptance 1 and 3; fake defaults,
  published exact pin and real-harness preflight. Finish M5.02's deferred live setup checks: validated
  effective `allowed_dirs`, configured narrowing, denied access, refresh/restart and retained old-session
  spending. Wire the staged grant display to the validated session without copying requested grants.

M5.03a is split before implementation: **a1 description relationships** (kinds/groups, unique
actions/types/ports, tool-owned type names and input-name collisions), then **a2 session/result/refusal
relationships** (effective-grant boundaries and continuity, invocation identity, contextual binding
errors). Each validator is tested against the published v0.4 schemas and independent fixtures before
the live caller is switched. Tests may compile the staged validator directly; live dispatch remains
exactly v0.3 until M5.03b, without a dual-version path. Each implementation commit includes its tests
and plan evidence within five authored files. M5.03b's caller/fixture/UI changes must be split by
dependency before building; generated assets and lockfiles stay separate. M5.02b CI confirmation is
required before the first local merge of this milestone; the coordinator confirmed run 41528363837
green on 2026-11-02.

M5.03a1's staged description validator now checks cross-field action/group/type/port relationships
and direct parameter/profile-input collisions. Six tests cover independent renamed peers, core-only
tools and structurally valid but inconsistent declarations. Production is not wired to it yet.
The 25 published v0.4.0 source/schema/contract files match `d1c49a7` byte-for-byte; the crate's explicit
include policy intentionally keeps fixtures in its repository. Independent fixtures continue to
validate against the published schemas. No packaging discrepancy was found.
Independent review approved a1 with the live-cutover follow-ups kept explicit: inspect parameter
names introduced by referenced/composed schemas and verify callable input/profile relationships;
switch the staged test module to the live module once it is wired. Direct-name checks alone do not
close acceptance 2. M5.03a2 is further split into **a2.1 session grants**, **a2.2 invocation identity**
and **a2.3 binding-refusal context**, each with its own tests and no more than five authored files.
**a1 checkpoint (2026-11-02):** initial test compilation failed before the validator existed; six
targeted tests now pass. Independent review found no blockers. The shared-lock full matrix passed:
fmt/clippy, 341 Rust tests (4 ignored), typecheck, 55 UI tests, 2 asset-tool tests, Storybook,
reproducible assets, release build and 226 browser tests (8 skipped). No live behavior or wire-version
gate changed. Temporary port 4187 config was removed; lockfiles and prebuilt assets are unchanged.
M5.03a2.1 stages session-grant validation: effective directories must be canonical existing
directories inside that child's requested inputs. Empty means deny all; order is immaterial, but
the set and session identity cannot change within one observed client. Only an explicit replacement
starts with no previous observation. Three filesystem tests cover narrowing, empty access, unrelated
paths, symlink escape, non-canonical/missing/file paths and same-session mutation. Live wiring and
old-session spend preservation remain in M5.03b.
The independent a2.1 review found no blockers. When wiring, pass only schema-validated current/prior
observations from the same owned child and that child's already-canonical argv grants. Existing
ledger checks still own tool identity, start time and budget continuity. An unexpected session ID
continues to require explicit reconnect, preserving the existing no-silent-reset policy. Directory
checks are point-in-time observations; the tool still enforces the path rule on each actual read.
**a2.1 checkpoint (2026-11-02):** three targeted tests pass after the initial missing-module failure.
The shared-lock full matrix passed fmt/clippy, 344 Rust tests (4 ignored), typecheck, 55 UI tests,
2 asset-tool tests, Storybook, reproducible assets, release build and 226 browser tests (8 skipped).
The temporary port 4187 config was removed; lockfiles/prebuilt assets are unchanged. This slice is
still staged validation; live session wiring and its browser acceptance remain in M5.03b.
M5.03a2.2 stages invocation identity checks after schema/association validation: a resolved invocation
must name the result's action, and images-profile delivery options cannot appear in replay arguments.
The validator does not rewrite arguments, defaults or opaque nested values. Unresolved invalid inputs
keep their schema-permitted null invocation. Portable input/reference validation and preservation
continue with the live cutover and M5.04–M5.08; this is not a claim of a complete replay bundle.
M5.03a2.3 stages contextual binding-refusal checks: declared submitted operation ports, mandatory
in-range indices only on list ports, one entry per failed item in port/item order, and a single
unresolved identity for artifact reads or exports. Human-readable expected/got/remedy fields remain
opaque. a2.2 and a2.3 have separate targeted tests; their joined verification/review covers five
authored files (two validators, two test files and this plan) before the next local merge.
Review regressions additionally cover integral JSON indices such as `1.0` and a schema-valid
non-binding refusal reaching the binding validator: both failed first, then passed after accepting
bounded integral numbers and rejecting other refusal reasons before reading bindings.
The initial joined matrix found an unrelated WebKit baseline test using raw label fills. Its trace
showed the second fill had not updated React's committed value before Run, so both edits now use the
existing shared committed-input helper; native editing remains covered separately. The saved-settings
assertions are unchanged, and 20 WebKit repetitions passed without retries or timeout changes. This
test-only correction is a separate commit from the validators and the version cutover.
**a2.2–a2.3 checkpoint (2026-11-02):** nine targeted Rust tests pass, including both review
regressions. Initial tests failed before their validator modules existed; the review regressions
failed on rejected integral indices and a non-binding panic before their fixes. Independent review
and follow-up found no blockers. Final shared-lock verification passed fmt/clippy, 353 Rust tests
(4 ignored), typecheck, 55 UI tests, 2 asset-tool tests, Storybook, reproducible assets, release build
and 226 browser tests (8 skipped). The baseline test also passed 20 WebKit repetitions. Temporary
port 4187 config was removed; lockfiles/prebuilt assets remained unchanged. The validators are still
staged; live invocation/refusal wiring follows the mechanical version cutover in a separate slice.

**M5.03b preparation — task invocation fixture:** before the mechanical cutover, add
`testkit/task_invocation.rs` and `tests/task_invocation.rs` (plus this plan). The helper records
resolved image references in declared slot order, applies the owned task schema defaults, retains
explicit caps and opaque parameter values, excludes delivery switches, and takes explicit effective
settings and requirement names. Missing image resolution is an error, never an invented reference.
It remains test-only until the version switch; this slice does not change live dispatch.

**Task invocation fixture checkpoint (2026-11-02):** four tests failed before the helper existed
and now pass against the published v0.4 invocation schema. Review approved; documented caller
preconditions require validated action/effective settings, actual resolved references in submitted
order, and names of requirements actually used. Full matrix passed: fmt, clippy, 357 Rust tests
(4 opt-in ignored), 55 UI unit tests, 2 asset tests, Storybook, byte-identical check-ui builds,
release build and 226 Chromium/WebKit tests (8 opt-in skipped). Live peers still use exactly v0.3.

**Task resolution preparation (split before merge):** the free-task slice changes exactly
`testkit/scenarios.rs`, `tests/fake_wire.rs`, `tests/attempt_execution.rs`,
`ui/tests/browser/cancellation-recovery.spec.ts`, and this plan. Resolve image identities before a
synthetic wait and preserve those inputs when cancelled. Any failure before resolution, including
file I/O, stays invalid even if cancellation arrives before saving, with no MCP reply to the
cancelled request. A deterministic rejection/cancel gate verifies this through the independent
stdio peer. Host and browser recovery checks now require complete matching input evidence rather
than the previous missing inputs. Partial-evidence scenarios remain independently covered.

The targeted stdio/recovery regressions pass. Review approved the preparation; the initial matrix
found the old recovery expectation and the subsequent browser pass reproduced its UI counterpart
before being stopped to apply that same correction. Final review approved; the full matrix passed:
fmt, clippy, 358 Rust tests (4 opt-in ignored), 55 UI unit tests, 2 asset tests, Storybook,
byte-identical check-ui builds, release build and 226 Chromium/WebKit tests (8 opt-in skipped).

**Result publication repair (before paid preparation merges):** the full matrix reproduced a
reader opening the free fake's `run.json` between file creation/truncation and its completed write,
leaving cancellation recovery unconfirmed with an EOF parse error. Replace direct result writes
with same-directory atomic publication. Slice files: `testkit/result_file.rs`,
`testkit/main.rs`, `testkit/scenarios.rs`, `tests/result_file.rs`, and this plan. A concurrent reader
must see an entire old or new document, never a partial one. The subsequent paid slice adopts the
same writer for its running and terminal snapshots. No retry/timeout increase or host workaround.

**Atomic publication checkpoint (2026-11-02):** the concurrent-reader regression failed with
the original direct write and passes with atomic publication; staging cleanup also passes.
Twenty repetitions each of that pair and host cancellation recovery passed under the shared lock.
Review approved. Full matrix passed: fmt, clippy, 360 Rust tests (4 opt-in ignored), 55 UI unit
and 2 asset tests, Storybook, check-ui, release build and 226 browser tests (8 opt-in skipped).

**Following preparation slice — paid input resolution:** `testkit/paid.rs`, `tests/fake_wire.rs`,
and this plan. Share actual grant, image/frame and fingerprint checks before any synthetic provider
reservation. The regression already demonstrated that a mismatched fingerprint was accepted;
the free-task slice merged at `0ac13bc`, and the isolated paid regression fails on that revision.
The paid path now records actual resolved references (including when no `expect` is supplied)
and refuses mismatches before reserving or charging synthetic spend. An input failure takes
precedence over a stale estimate when both apply. Running and terminal snapshots now use the
shared atomic result writer; paid cancellation recovery is repeated before the full matrix.
No live host behavior changes or additional version support is introduced by either slice.
Both new regressions fail against the old implementation and pass with the shared resolver.
Review approved the final three-file slice. Twenty paid cancellation repetitions passed, followed
by the full matrix: fmt, clippy, 362 Rust tests (4 opt-in ignored), 55 UI unit tests, 2 asset tests,
Storybook, reproducible check-ui builds, release build and 226 browser tests (8 opt-in skipped).

**Checkpoint A file-budget exception (owner/coordinator, 2026-11-02):** one atomic final mechanical
cutover commit may exceed five authored files. List its exact files here before building it. It may
contain only the version/pin, strict gates and corresponding fixture/test-document switch; no other
behavior rides along. Runtime validation/display changes stay in their own slices before or after.
Full review, full matrix and strict version-gate tests still apply. This is a one-off, not a budget
change for later tasks.

**Exact mechanical cutover inventory (recorded before building M5.03b):**
The following files form that single atomic commit. Changes are the exact published pin/alias removal,
version gates, required v0.4 wire fields in existing fake documents, matching test/story documents,
version-neutral names for the former control-only fake, and current-version documentation. New
runtime semantics, grant-display wiring, callable-schema checks and real-peer preflight behavior are
excluded and land separately. Existing saved-history cases remain; review adds v0.3 to the
existing storage-read regression without changing storage behavior.

- Dependency: `Cargo.toml`, `Cargo.lock`.
- Host gates/test fixtures: `src/tools/mod.rs`, `src/tools/session.rs`,
  `src/tools/dispatch_transport.rs`, `src/tools/render/process_tests.rs`, `src/tools/render/tests.rs`.
- Existing fake documents/callers: `testkit/fixtures.rs`, `testkit/main.rs`, `testkit/estimates.rs`,
  `testkit/readiness.rs`, `testkit/scenarios.rs`, `testkit/paid.rs`, `testkit/v04_calls.rs`, `testkit/v04_schemas.rs`,
  `testkit/v04_store.rs`; rename `testkit/v03_control.rs` to `testkit/control_peer.rs` and
  `testkit/v03_fixtures.rs` to `testkit/control_fixtures.rs`. The prepared task-invocation helper
  is wired to existing result construction here; its logic/tests land separately beforehand.
- Rust contract tests: `tests/attempt_storage.rs` (extend the existing old-history loop with v0.3),
  `tests/contract_fixtures.rs`, `tests/description_semantics.rs`,
  `tests/mcp_client.rs`, `tests/fake_wire.rs`, `tests/rascal_smoke.rs`, `tests/squint_smoke.rs`, `tests/tool_session_api.rs`,
  `tests/v04_bindings.rs`, `tests/v04_contract_fixtures.rs`, `tests/v04_results.rs`, `tests/v04_runs.rs`,
  `tests/v04_schemas.rs`, `tests/v04_wire.rs`, and the preparatory `tests/task_invocation.rs`.
  Rename `tests/v03_fixtures.rs` to `tests/control_fixtures.rs`, `tests/v03_wire.rs` to
  `tests/control_wire.rs`, and `tests/support/v03_schema.rs` to `tests/support/contract_schema.rs`.
- UI version gates/documents: `ui/src/api/tool-session.ts`, `ui/src/api/workspace.ts`,
  `ui/src/features/actions/model.ts`, `ui/src/features/workspace/CostReview.stories.tsx`,
  `ui/src/features/workspace/Evidence.stories.tsx`, `ui/src/features/workspace/ToolSetup.stories.tsx`,
  `ui/tests/browser/real-peer-reports.spec.ts`, `ui/tests/browser/squint.spec.ts`,
  `ui/tests/unit/action-form.test.ts`, `ui/tests/unit/estimates.test.ts`,
  `ui/tests/unit/image-selection.test.ts`, `ui/tests/unit/tool-session.test.ts`,
  `ui/tests/unit/workspace-contract.test.ts`.
- Current-state docs: `README.md`, `ENGINEERING.md`, `docs/integration-sources.md`, `testkit/README.md`,
  and this `PLAN.md`. Historical review evidence is not rewritten.

The inventory includes `testkit/paid.rs` and `tests/fake_wire.rs` before cutover edits: the paid
result must wire the prepared invocation helper, and the independent wire assertions must verify
its required v0.4 field. No new execution behavior is included in those mechanical edits.

UI-only story/test documents are partial projections, not full wire fixtures: session grant
fields and task kinds are checked on the Rust wire boundary. Their UI types/parsers and complete
grant projections land in the separate semantic/display slice. This avoids adding parser behavior
to the mechanical switch. Saved v0.3 storage/API reads and refusal to launch its renderer have
explicit regressions; no old evidence is rewritten or given an invented invocation.

Generated `ui/prebuilt` output remains a separately reviewed build commit. If a further authored
file is required, amend this inventory with its reason before editing it; do not turn the exception
into unrelated cleanup. Strict tests must demonstrate v0.4 acceptance and v0.3 live rejection while
saved older records remain readable. Preparatory task-invocation generation is independently tested
against v0.4 before the fake's version switch; no temporary live compatibility path is introduced.

**Mechanical cutover checkpoint (2026-11-02):** strict Rust and browser version-gate tests failed against
v0.3 before the switch. The published runtime pin now replaces the dev alias; existing peer
and test documents use v0.4, required task kinds, session grants and the already-reviewed invocation
builder. Control-only fixtures use version-neutral names. Cross-document semantic wiring, live
effective-grant display and real-peer preflight remain separate slices; this commit does not claim
M5.03 or checkpoint A complete. Independent review and follow-up approved the change after
correcting current-version documentation and adding old-evidence regressions. The final shared-lock
matrix passed fmt/clippy, 364 Rust tests (4 opt-in ignored), typecheck, 56 UI unit tests, 2 asset
tests, Storybook, byte-identical check-ui builds, release build and 226 Chromium/WebKit tests
(8 opt-in skipped). Saved v0.3 storage/API reads preserve the original record; renderer tests reject
either an old declaration or an old result before starting a process. Generated assets are committed
separately; the temporary port 4187 browser config was removed.

**Post-cutover semantic slices:** wire the staged validators separately from the approved
mechanical switch. First, **b1 description boundary** changes `src/tools/mod.rs`,
`tests/description_semantics.rs`, `ENGINEERING.md`, and this plan: existing relationship tests
must exercise the public live validation boundary and fail there before activation. The boundary
checks the structural schema before calling the semantic validator, so malformed input cannot
reach its schema-dependent assumptions. Existing task and operations fixtures must still pass.
This activates the reviewed direct-name relationships only; composed/referenced parameter-name
checks, callable-schema relationships, invocation/refusal wiring and effective session grants
remain explicit follow-ups before checkpoint A. Each slice needs independent review and the
full matrix; the atomic file-budget exception has ended.

**b1 checkpoint (2026-11-02):** four relationship tests failed through the public live boundary
before activation; all seven description tests now pass, including malformed-document rejection
before schema-dependent checks. Independent review approved. The shared-lock full matrix passed
fmt/clippy, 365 Rust tests (4 opt-in ignored), typecheck, 56 UI unit tests, 2 asset tests, Storybook,
byte-identical check-ui builds, release build and 226 Chromium/WebKit tests (8 opt-in skipped).
The temporary port 4187 config was removed; lockfiles and prebuilt assets are unchanged. Discovery
and renderer resolution now enforce the same direct description relationships. The composed-schema,
callable-schema and remaining semantic/display follow-ups above are still open.

**Cutover CI teardown correction (2026-11-02):** a separate three-file slice
(`tests/mcp_client.rs`, `testkit/main.rs`, this plan) corrects the invalid-registration test's shutdown
assumption. A saved terminal preservation record does not imply that its worker has released
the client preservation guard; `close()` is one bounded step and deliberately defers in that
window. The cutover did not change this test body or shutdown ownership, and the failed
configuration probe is closed before its declaration error is stored. Use the same
`begin_shutdown` / quit-watch completion signal as the host, then check stopped phase,
zero active attempts, disposable-input cleanup and the final saved evidence. Add an explicitly gated resource
case alongside the invalid registration to exercise shutdown during preservation without a
scheduling-delay assumption.
Keep the preservation guard and force-stop policy unchanged. Require a failing reproduction,
stress runs, independent review and the full matrix before merging this correction.

**As built — cutover CI teardown correction (2026-11-02):** the original timing window did
not recur in 200 local repetitions. A held journal read reproduced the old positive `close()`
assertion failure after the test confirmed the peer had reached its preservation gate. The
corrected regression proves one close step defers, releases the gate after shutdown begins,
and waits for the host's completion signal before checking final evidence and cleanup. Pairing
this with a rejected registration also verifies discovery isolation during shutdown. Independent
review approved after correcting the initial gate to target the journal actually read on success.
The final two cases each passed 200 repetitions. The full shared-lock matrix passed fmt/clippy,
366 Rust tests (4 opt-in ignored), typecheck, 56 UI unit tests, 2 asset tests, Storybook,
byte-identical check-ui builds, release build and 226 Chromium/WebKit tests (8 opt-in skipped).
Lockfiles and generated assets are unchanged; the temporary browser-port config was removed.
This is a test-only lifecycle correction; the remaining M5.03 gates are still open.

**Checkpoint A:** merge M5.01–03 as a coherent cutover of validation, launch arguments and fake
defaults; v0.4 is the only supported live contract afterward. Green matrix, an independent fake
task run and the published exact dependency pin are required.
Rollback to `851b850` requires its v0.3 peers and saved pre-M5 registrations. It is safe on the
same library only before a storage migration; afterward use a compatible binary or restore the
matching pre-upgrade backup while retaining newer evidence separately. Revert cutover dependents
together, never mix wire versions.

#### M5.04 — Preserve resolved invocations

- [ ] Snapshot result invocation on every terminal outcome and show it in the existing inspection
  drawer alongside, and distinctly from, the recorded request. Null invalid invocation is explicit.
- [ ] Preserve defaults, resolved effective settings, caps and requirement names exactly; no host
  inference from UI labels, no credential values, no substituting current draft state for a saved run.
- [ ] Older attempts remain viewable with invocation unavailable; any storage migration is additive,
  versioned and tested against a pre-M5 library before the new binary opens real user data. Back up
  the database and registration state before the first real-data upgrade here or in any later task;
  document compatible-binary/backup rollback and refuse unsafe downgrade, without deleting newer evidence.

**Depends:** M5.03. **Areas:** attempt persistence, result preservation, Inspector and API types.
**Verify:** completed/invalid/cancelled fixtures, history reopen and draft-edit isolation.
**Commit:** `feat: preserve resolved run invocations`.

#### M5.05 — Present the declared activity hierarchy

- [ ] Playground/Operations/Utilities and the focused grouped picker derive only from kind/group
  metadata. Tools with only tasks stay uncluttered; unavailable operations explain readiness normally.
- [ ] Switching tool/kind/action resets the existing draft and estimates, while leaving saved
  runs immutable. M5.07 extends this same behavior to newly introduced port bindings. Parameter labels/choices come from the current declaration.
- [ ] Add keyboard, long-label, empty-kind and narrow-screen stories plus stable region snapshots
  using the V19 visual policy; record intentional differences from the two source mocks.

**Depends:** M5.03. **Areas:** Workspace, Picker, styles/stories/browser tests.
**Verify:** fake with renamed/reordered groups; keyboard selection and no horizontal category strip.
**Commit:** `feat: organize activities by declared kind and group`.

#### M5.06 — Bind operation ports through normal preparation

- [ ] Bind scalar/many/optional ports in order, with image input/ref and shared location value/ref
  alternatives; keep task image/location slots on their images-profile representation. Optional empty ports are omitted; populated many ports are nonempty arrays.
  Ports cannot be overwritten by params; validate final arguments against the advertised MCP schema.
- [ ] Use normal preparations, estimates, bound paid calls, progress, cancel and dispatch. Changing a
  port value/order invalidates estimates. Do not compute the tool's identity or binding digest.
- [ ] Decode binding refusals by port and optional item index, retaining order and remedy. They create
  no tool run/spend and release client_run_id; malformed inline input can instead produce an invalid
  run. Do not conflate those cases or automatically resubmit.

**Depends:** M5.03. **Areas:** bindings, prepared/estimate/dispatch, submission/API models.
**Verify:** binding refusal from both estimate and call, all four binding problems, many-index
errors, ID reuse after refusal, paid-port edits.
**Commit:** `feat: bind typed operation inputs`.

**Checkpoint B:** invocations survive reopen; hierarchy is navigable; fake operations use the existing
run path without weakening paid guards. Revert M5.04–06 newest dependent first if needed; retained
new evidence remains data, not a reason to dispatch through an older contract.

#### M5.07 — Add port editors to the input pane

- [ ] Render typed port rows before parameters, with image/location editors and a saved-artifact
  selection affordance. Many rows support accessible reorder/remove and item-specific refusal text.
- [ ] Required, optional, empty, resolving and refused states are distinct. Artifact selection does
  not claim frame validation; unknown compatibility is visible before explicit Run.
- [ ] Input remains scrollable with fixed Run footer on desktop; long forms and validation focus
  work at narrow width and increased text size. Stories cover all port shapes without tool branches.

**Depends:** M5.05–06. **Areas:** port editor component, form state, API adapter, stories/browser tests.
**Verify:** fresh image/location operation and scalar/list native binding through a fake reference.
**Commit:** `feat: edit operation ports in the workspace`.

#### M5.08 — Index output references and inspect artifacts

- [ ] Preserve `artifacts.in/out` occurrence order, references and preview assets independently;
  enforce tool/resource/identity consistency and advertised type/version rules. Reconcile duplicate
  images/locations profile entries without duplicating owned image bytes or presenting phantom outputs.
- [ ] Read only the advertised artifact resource and preview, with bounded parsing and clear missing,
  stale-session and malformed-resource states. Display native record as opaque data; never decode it.
- [ ] Output cards support multiple ports and previewless outputs, including salvaged error/cancelled
  outputs. Preview failures affect preservation, not the tool's result or the native artifact identity.

**Depends:** M5.04,06–07. **Areas:** artifact resource client, evidence/persistence, result cards/Inspector.
**Verify:** restart resource resolution, artifact-resource refusal with port:null, forged URI/identity,
missing preview and multiple same-port outputs.
**Commit:** `feat: inspect typed operation outputs`.

**Planned build slices** (ordered, each with its own targeted checks and matching docs):

- **M5.08a: artifact evidence and resource access** — acceptance 1–2; persistence/resource/refusal
  tests.
- **M5.08b: output cards and inspection** — acceptance 3; stories and result browser tests.

#### M5.09 — Connect outputs to new drafts

- [ ] Send to and the saved-artifact picker use declared compatible ports; choosing among multiple
  destinations is explicit. Carry source attempt, output port and occurrence as host lineage metadata.
- [ ] Fill the destination draft without Run, hidden import or upstream work. Preserve list ordering
  and duplicate occurrences; a wrong frame stays a tool refusal on the exact receiving port.
- [ ] Shared image transfers use owned bytes/expect when a receiving store cannot resolve the ref;
  location transfers use the shared contract representation or explicit supported import. Never send
  a foreign native type or parse an opaque record to fabricate a shared value.

**Depends:** M5.07–08. **Areas:** artifact selection, Workspace draft state, bindings/lineage persistence.
**Verify:** fake Line → Rasterize → Feather equivalent, output-to-two-ports, cross-tool shared image,
no hidden calls; stories and snapshots. **Commit:** `feat: send saved artifacts into operation drafts`.

**Checkpoint C:** the independent-pieces loop works end to end with fake references. Saved outputs
can be selected after host restart; missing tool-store data is honest and never triggers reconstruction.
Rollback removes the UI route first, retaining immutable references and previews; apply the
first-migration backup/downgrade rule from M5.04.

#### M5.10 — Store native bytes with reference-aware ownership

- [ ] Add a bounded opaque blob store and versioned library links for native exports, artifact
  occurrences and future bundles; digest-check publication and deduplicate bytes without decoding them.
- [ ] Attempts/examples/bundles own references independently. Deletion preview identifies bytes
  actually eligible for removal; deleting one attempt never removes another owner's artifact inputs.
- [ ] Migration/reopen and interrupted publication tests preserve existing assets/history. Cleanup
  touches only host-owned blobs, never a tool workspace or returned export file.

**Depends:** M5.08. **Areas:** library migration, blob publication, examples/cleanup reference accounting.
**Verify:** shared-byte ownership, failed publication and pre-M5 database upgrade tests.
**Commit:** `feat: preserve native artifact bytes in the library`.

**Planned build slices** (ordered, each with its own targeted checks and matching docs):

- **M5.10a: blob storage and migration** — acceptance 1 and storage portion of 3; upgrade/publication
  tests.
- **M5.10b: ownership and cleanup integration** — acceptance 2 and cleanup portion of 3;
  shared-ownership/deletion tests.

#### M5.11 — Add artifact export/import controls

- [ ] Use reserved controls only when advertised by portable artifact types; controls remain responsive
  during runs, create no attempts and do not take paid-run permits. Apply request size/concurrency bounds.
- [ ] Read the exact exported file, verify declared bytes/hash/media type, then publish owned bytes.
  An export outside evidence_dirs is refused before opening, even with a correct advertised hash.
  For import, stage under an allowed directory or bounded inline form; verify returned artifact and
  already_present semantics. Wrong native format is never retried as a preview image.
- [ ] Missing export refs and invalid imports surface structured refusal/remedy. Cancellation/disconnect
  leaves incomplete local state unpublished; an interrupted tool import is reconciled by idempotent retry,
  not falsely described as rolled back.

**Depends:** M5.02–03,10. **Areas:** artifact controls, exact-file reader, staging and API endpoints.
**Verify:** control concurrency, duplicate import, corrupt hash, unsupported type and unsafe returned paths.
**Commit:** `feat: transfer native artifacts through contract controls`.

#### M5.12 — Preserve reusable artifacts after runs

- [ ] Queue bounded deduplicated preservation for portable non-image artifact inputs/outputs after their references
  are known. Existing successful run state is never held hostage by export failure or a stopped peer.
- [ ] Show preserved, preserving, workspace-dependent and failed states with explicit retry; portable:none
  has a reason. Restart can resume user-requested preservation without replaying the original action.
- [ ] Reopen an unresolved saved artifact by offering import of its verified owned bytes; validate the
  restored identity and re-read the artifact resource before binding. Do not silently import or run.

**Depends:** M5.08,10–11. **Areas:** preservation queue/status, result cards/Inspector, artifact selection.
**Verify:** tool store removed after preserving both a JSON location and native Mask, interrupted export,
duplicate outputs and bounded queue.
**Commit:** `feat: preserve reusable operation artifacts`.

#### M5.13 — Save reusable operation examples

- [ ] Examples retain typed bindings, parameters, owned artifact/image references and source occurrence
  metadata independently from the original attempt. Opening fills a draft and never executes anything.
- [ ] Missing resources, importable owned bytes and genuinely unportable bindings are distinguished;
  saving partial preservation requires an explicit workspace-dependent label, not a silent success claim.
- [ ] Deleting the source run and restarting leaves a preserved example usable. Moving the tool store
  uses its new registration/resource identity; the host never searches old folders.

**Depends:** M5.09,12. **Areas:** examples persistence/API, Examples drawer, cleanup UI.
**Verify:** saved Feather inputs with changed sigma run without upstream calls; missing-store import flow.
**Commit:** `feat: save independent operation examples`.

**Checkpoint D:** preserved native examples survive deletion/restart; tool control failures cannot
change run outcomes or accounting. Back up the database before schema adoption; binary rollback
requires compatible schema or the matching backup, never a silent downgrade or ledger reset.

#### M5.14 — Export the resolved single-call handoff

- [ ] Pipeline drawer offers resolved invocation, recorded MCP evidence, requirements and a supported
  `<tool> call <action> --arguments <file.json>` handoff. Materialize owned image locators and explicit
  `--allow-dir`; shell-quote displayed argv, use argv arrays internally and never execute copied code.
- [ ] Include resolved defaults/caps/effective values from the selected saved run, not current config.
  Required config/env names are setup instructions; missing secrets are not inserted into downloads.
- [ ] Label this handoff dependent on the configured local store. Use the contract-standard --config and --allow-dir arguments; never blindly copy serve-only
  argv or interpret private effective fields. If the serving store requires other tool-specific launch
  arguments, show that unresolved local dependency and withhold a claimed ready-to-run command.
  A fresh store needs explicit native import or the bundle route. Distinguish this from portable; unresolved/null invocation and old
  evidence retain inspection/export with the runnable option unavailable and explained.

**Depends:** M5.04,12. **Areas:** invocation handoff builder, Pipeline/API, stories and tests.
**Verify:** paths with spaces/metacharacters, missing owned bytes, draft edits and null invocation.
**Commit:** `feat: export resolved CLI invocations`.

#### M5.15 — Write canonical invocation bundles

- [ ] Build schema-valid single-step manifests with original result/invocation, ordered image inputs and
  non-image artifact inputs (including shared location references). Store original image bytes and exact
  tool exports under digest-based names; map repeated
  slot/port entries by occurrence and preserve duplicates without duplicating archive members.
- [ ] Produce byte-identical uncompressed ustar: manifest first, remaining members sorted, prescribed
  headers/modes/owners/timestamps. Check image digest and native export hash; never substitute previews.
  Use a fixed media-type/extension mapping, lowercase safe basenames and the peer-compatible 96-byte
  member-name bound; never take an archive member name from an export path.
- [ ] Enforce documented limits before writing (initial common CLI-compatible ceiling: 8 MiB manifest,
  64 MiB per member, 64 members, 256 MiB total; over-limit exports explain the limit). Include inputs by
  default; explicit omissions populate reference_only, failed/unportable exports populate unportable.

**Depends:** M5.04,10–13. **Areas:** new bundle writer/manifest mapping, archive dependency, library links.
**Verify:** schema and byte-stability golden tests, Line → Rasterize with a location reference and
its JSON export, location-by-value contrast, list duplicates, deterministic ordering, size boundaries.
**Commit:** `feat: assemble portable invocation bundles`.

#### M5.16 — Expose saved bundles in the pipeline drawer

- [ ] Export reviews included files, omissions, artifact dependencies and requirements before downloading.
  Portable versus workspace-dependent status is derived from preservation, never a user-editable claim.
- [ ] Save manifest/digest/member ownership and selected-attempt links atomically; reopening a bundle
  returns identical bytes. Failed assembly leaves no completed bundle or leaked ownership reference.
- [ ] Pipeline drawer keeps command/MCP/configuration/input concepts from journey; evidence archive and
  invocation bundle have distinct descriptions. Offer `<tool> call --bundle` only for a single-tool
  bundle with empty reference_only and unportable lists; otherwise explain required host setup/replay.
  Add loading/error/partial stories and region snapshots.

**Depends:** M5.14–15. **Areas:** bundle API/persistence, Pipeline drawer, cleanup ownership display.
**Verify:** export selected saved evidence after draft change, interrupted download, source deletion.
**Commit:** `feat: review and download saved invocation bundles`.

**Checkpoint E:** single-run invocation and bundle are reviewable, stable, and labelled accurately.
Rollback can remove new export routes without deleting already downloaded files or retained evidence.

#### M5.17 — Validate bundles before importing

- [ ] Parse uploaded bundles with bounded uncompressed archive validation: safe member names, regular
  files only, no links/traversal/duplicate members, complete manifest/schema/hash/reference checks before
  any tool import, file publication or action dispatch. Apply M5.15 limits and reject unsupported versions.
- [ ] Show steps, tools, requirements, omitted inputs and workspace dependencies in an import review.
  Supplied replacements become a new draft; never mutate the original archived invocation/result.
- [ ] Only explicit import stages owned bytes and calls the needed tool controls; imports are idempotent,
  failures report which completed. Tool installation/configuration/grants remain separate explicit setup.

**Depends:** M5.11,15–16. **Areas:** bundle reader, import staging/control orchestration, drawer/API.
**Verify:** adversarial archive corpus, no side effects on invalid bundle, failure between two imports.
**Commit:** `feat: review invocation bundles before import`.

**Planned build slices** (ordered, each with its own targeted checks and matching docs):

- **M5.17a: bounded archive reader** — acceptance 1; malformed archives cause no effects.
- **M5.17b: import review and orchestration** — acceptance 2–3; drawer and partial/idempotent import
  tests.

#### M5.18 — Replay one imported step through the workspace

- [ ] A reviewed imported step opens a normal draft with fresh resource refs and materialized inputs.
  It reuses ordinary readiness/estimate/cap/Run/cancel; recorded estimates or session ids never authorize it.
- [ ] A new attempt records linkage to the source bundle while preserving its original evidence.
  Identity differences are displayed, not equated with failure or a guarantee of deterministic judgments.
- [ ] Reference-only/unportable inputs block Run until resolved explicitly. Import, opening or restoring
  a draft cannot start a paid action; restart requires new session readiness and estimates.

**Depends:** M5.06,09,17. **Areas:** replay draft adapter, Workspace/Pipeline state, attempt linkage.
**Verify:** original store absent, same invocation with new paths, changed provider output, paid confirmation.
**Commit:** `feat: replay imported invocations as new experiments`.

#### M5.19 — Export selected experiment chains

- [ ] Select an ordered set of saved attempts, exporting only those steps, never hidden provenance.
  Conservatively carry each non-image artifact input's export or mark it unportable, even when a selected preceding
  step also produced it. Do not claim this extra copy replaces fresh-output rebinding.
- [ ] Use one manifest-edge resolver for export and import/replay: find earlier matching output
  occurrences for every artifact-reference input (image, shared location and tool-native), including
  artifacts.out and images.out. Coalesce
  only demonstrably mirrored profile entries for the same output, never distinct ports/items with equal
  identities. Zero matches means an external supplied input; one match means a fresh-output edge;
  multiple matches are ambiguous. Export only if this resolution agrees with recorded host lineage;
  a selected producer with the same identity as an unselected actual source must not silently replace it.
  Refuse ambiguous/mismatched runnable chains with an explanation and single-step bundle alternative.
- [ ] Preserve selected order and deterministic bytes. Imported bundles derive edges from the manifest
  using the same resolver, without requiring private host lineage; ambiguous ones cannot replay as a
  chain. No invented from_step fields. Mixed-tool bundles use host replay, not a single-tool CLI claim.

**Depends:** M5.09,15–18. **Areas:** chain selection/manifest builder, bundle ownership, Pipeline drawer.
**Verify:** two consumers, two ports/items with equal identities, mirrored profiles, image edges,
Line → Rasterize by reference versus by value, selected producer/unselected source collision,
omitted upstream step and mixed tools.
**Commit:** `feat: bundle selected experiment chains`.

#### M5.20 — Replay chains using fresh outputs

- [ ] Use M5.19's manifest resolver to map each earlier-output input to this replay's output port and
  occurrence, never the old identity; report nondeterministic changes. Missing/ambiguous outputs stop the chain before a consumer
  runs. Include shared location refs in this rule; location values remain literal inputs. Fresh image
  outputs are copied through owned assets into the granted staging directory before consumption, never
  passed directly from an evidence-only run directory. Maintain explicit host lineage without modifying
  the contract bundle format.
- [ ] Present ordered step progress in the shared drawer. Each step becomes its own ordinary attempt;
  paid steps require a fresh estimate and explicit Run review after their inputs are known. No blanket
  reuse of a cap approval, automatic retry or silent reconstruction after a failure.
- [ ] Stop/cancel prevents later steps, retains completed evidence and spend, and offers explicit resume
  from verified saved outputs. A session restart invalidates pending preparation normally.

**Depends:** M5.18–19. **Areas:** bounded replay state machine, draft/dispatch adapter, progress drawer.
**Verify:** changed producer identity, many output order, refusal, cancellation and paid next-step pause.
**Commit:** `feat: replay chains with freshly produced artifacts`.

**Planned build slices** (ordered, each with its own targeted checks and matching docs):

- **M5.20a: replay orchestration and cancellation** — acceptance 1 and dispatch portion of 2–3; fake
  changed-output/paid/cancel tests.
- **M5.20b: step progress and resume UI** — presentation portion of 2–3; stories and keyboard/browser
  replay tests.

**Checkpoint F:** fresh-store single and multi-step replay pass the independent fake, including a
nondeterministic producer. Remove replay entry points before reverting orchestration; stop active work
and preserve partial attempts/charges. Do not label a partially executed import/replay transactional.

#### M5.21 — Verify real v0.4 peers

- [ ] Build the approved Rascal/Squint revisions in isolated disposable checkouts under the shared lock.
  Verify version/description/allowed_dirs against v0.4 and document the exact executable and dependency
  revisions. Never edit sibling branches; ask the coordinator for any necessary replacement revision.
- [ ] Rascal: Line → Rasterize by location reference → Feather (including the Location JSON export),
  location-by-value contrast, saved Mask parameter edit without upstream calls, indexed
  binding refusal, resource after restart/store move, export/import to an empty store, and both host and
  `rascal call --bundle` replay with original workspace unavailable. Assert invocation and native identity
  for deterministic fixtures; do not claim visual previews prove native equivalence.
- [ ] Squint: task-only hierarchy, ordered list images, invocation and single-tool CLI bundle replay
  against its scripted test provider. Confirm path denial without grants and success with staging grants.
  Real-provider checks, if used, keep explicit cap/spend records and do not replace deterministic checks.

**Depends:** M5.03–20; published pin and peer v0.4 availability for final acceptance.
**Areas:** opt-in real-peer harness, browser integration, fixtures and integration-source docs.
**Verify:** complete journeys in Chromium/WebKit; strict report console checks remain; record native
Safari manual coverage separately, never substitute WebKit as proof of Safari.
**Commit:** `test: verify v0.4 operation and bundle journeys`.

**Planned build slices** (ordered, each with its own targeted checks and matching docs):

- **M5.21a: Rascal integration** — acceptance 1–2 for Rascal; isolated operations/CLI/browser checks.
- **M5.21b: Squint integration** — acceptance 1 and 3 for Squint; isolated task/list/CLI/browser checks.

#### M5.22 — Close out the operations milestone

- [ ] Update SPEC/INTERFACE/ENGINEERING, tool requirements/handoffs and user setup/export docs to the
  implemented behavior; retire resolved C1/C2/R3 gates with evidence, retain actual distribution gaps.
  Capture before/after mock comparisons, stories and region screenshots for changed screens.
- [ ] Confirm the published `=0.4.0` pin and packaged-schema verification from M5.03 remain intact,
  and run the complete PLAN matrix. Include asset-manifest unit checks,
  reproducible prebuilt assets/check-ui and release build, plus targeted repeated tests only where a
  lifetime/race change needs evidence. Hosted CI must be green before declaring completion.
- [ ] Record peer revisions, fixtures, archive bounds, ambiguous-chain limitations, preservation failures,
  path migration instructions and downgrade limits in an M5 review. No implicit approval of M7 shipping.

**Depends:** M5.21. **Areas:** milestone review, SPEC/INTERFACE/ENGINEERING and handoff/user docs.
**Verify:** documentation links, final independent review, full matrix and coordinator-hosted CI.
**Commit:** `docs: record v0.4 operations acceptance`.

**Planned build slices** (ordered, each with its own targeted checks and matching docs):

- **M5.22a: published dependency and final matrix** — acceptance 2; packaged-schema verification, exact
  crates.io pin and full checks.
- **M5.22b: acceptance documentation** — acceptance 1 and 3; review links, coverage and hosted CI
  evidence.

### Approval, checks and remaining coordination

The owner approved `plan-m5-v04` on 2026-11-02, including the published v0.4 pin update.
The coordinator supplied the main revisions recorded above. Merge this plan locally, then follow
the dependency order above on short branches. Every coherent slice runs the
full matrix under `../wafflemaker/scripts/heavy.sh wafflery <command>` and receives review before
local merge; the coordinator alone scans, pushes and watches hosted CI. A red main takes precedence
before subsequent merges. UI slices include Storybook, keyboard/error states and Playwright regions
using the existing shared visual policy; do not loosen tolerances to hide layout changes. Update
matching docs in each code commit, not only at closeout. Plan-only edits use diff/link/consistency
checks rather than rerunning unchanged code.

No new shared-contract ruling or publication wait is needed. Owner approval is recorded above;
real-peer revision changes remain coordinator-owned. Ambiguous chain provenance is deliberately refused under
the existing manifest rather than extending it locally; broader chain expressiveness would need a
coordinator ruling. Portable Plan/Series/BandProfile, artifact enumeration, hidden provenance replay,
agent orchestration, evaluation suites and distribution remain outside M5. The host only lists
artifacts it has seen or explicitly imported; it does not promise a browser for an entire tool store.

## Verification and commit strategy

At each task run its targeted checks. At M0/M1/M2/M3 run the checks relevant to code now present:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
bun run --cwd ui typecheck
bun run --cwd ui test:unit
bun test scripts/ui-manifest.test.ts
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
| Contract version changes / path grants | M5.01–03 adopt exact v0.4 and replace MCP roots with explicit child grants, preserving separate evidence-read boundaries; MCP revision stays 2025-11-25 |
| Shared SDK conceals protocol assumptions | Independent fake wire peer; explicit protocol selection and manual child ownership |
| UI fixtures accidentally become tool-specific logic | Keep fixtures in Storybook/testkit; test differently named tools; validate contract objects |
| Saved evidence is mistaken for full reproducibility | Separate preservation categories and label exports accurately; M5 verifies native C2 portability, while completed C3 report checks remain regressions |
| Stable tool workspaces grow beyond movable library | Show storage locations separately; cleanup/relocation never guesses ownership |
| Too much infrastructure before a working screen | M1 import loop and M2 execution loop are visible checkpoints; no speculative packages/services |

M0–M3 and the V01–V19 client are complete. The R3 operations profile and C2 portable invocation
contract are specified in v0.4; host adoption awaits approval of M5 above and its real-peer
checkpoint. C3's v0.3 delivery is verified by V14–V18 against Rascal M12/Squint M9. R2 remains optional. Do not imply an Actions-only
development build satisfies those release requirements. Product choices remain Q1–Q19.
