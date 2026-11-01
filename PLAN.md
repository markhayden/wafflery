# Wafflehouse foundation implementation plan

Status: M0–M3 approved by the owner on 2026-10-31; implementation started.
Current implementation evidence: [M0 checkpoint](docs/reviews/2026-10-31-m0-checkpoint.md) and
[M1 checkpoint](docs/reviews/2026-10-31-m1-checkpoint.md), and
[M2 checkpoint](docs/reviews/2026-10-31-m2-checkpoint.md), and
[M3 checkpoint](docs/reviews/2026-10-31-m3-checkpoint.md). M0–M3 are complete: automated and
packaged-source checks pass, and the owner explicitly confirmed the native Safari checklist.
M4's explicit real-Rascal entry slice is implemented; automatic discovery and provider setup
remain follow-on work. See its [checkpoint](docs/reviews/2026-10-31-m4-entry-checkpoint.md).
The [current review disposition](docs/reviews/2026-10-31-plan-review-disposition.md) records
changes and validation limits; it is not implementation verification.
The owner agreed to proceed with foundation planning independently of later tool interfaces.
The owner authorized continuing into real-tool integration on 2026-10-31; the concrete next
slices are in [M4: real tool entry](docs/plans/m4-real-tool-entry.md). Paid runs are also authorized
once estimates/caps/session handling are implemented; the current dispatch guard remains offline-only.
Publishing and production installation are not authorized by this continuation.

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

M0–M3 do not wait for all C1–C4/H1 features. Wafflehouse owns the proposals that unblock the
other tools: [H1 launcher](docs/handoffs/H1-launcher-proposal.md) and
[C4 v0.2 semantics](docs/handoffs/C4-v0.2-proposal.md) are drafted now, with a ready-to-apply
[table-manners proposal patch](docs/handoffs/table-manners-v0.2-proposals.patch).
Owner/schema agreement is pending; do not treat the drafts as accepted wire interfaces.
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

- **Accept:** Cargo embeds existing assets without Node/Bun. Missing assets always fail. Stale assets warn in debug but fail in release or WAFFLEHOUSE_VERIFY_UI=1; CI always checks freshness.
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

### F05b — Detach and attach the Wafflehouse launcher

- **Accept:** Default launcher starts/attaches, waits for readiness then opens the browser. Two launches converge; terminal/tab close leaves host alive; idle Quit stops it.
- **Files:** `src/host/launch.rs`, control/main wiring, lifecycle tests, README.
- **Verify:** MacOS concurrent launch, terminal exit, invalid override, incompatible host and explicit idle quit; no real installer.
- **Depends:** F05a. **Commit:** `detach and attach the Wafflehouse launcher` with the appropriate conventional prefix.

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
cargo build --locked --release --bin wafflehouse
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
| Contract moves to v0.2 / roots deprecated after current MCP revision | Exact supported contract set, tagged-schema upgrade commit, array-capable host bindings; no MCP revision bump until replacement path-access rules are agreed |
| Shared SDK conceals protocol assumptions | Independent fake wire peer; explicit protocol selection and manual child ownership |
| UI fixtures accidentally become tool-specific logic | Keep fixtures in Storybook/testkit; test differently named tools; validate contract objects |
| Saved evidence is mistaken for full reproducibility | Separate preservation categories and label exports accurately; C2/C3 gates remain |
| Stable tool workspaces grow beyond movable library | Show storage locations separately; cleanup/relocation never guesses ownership |
| Too much infrastructure before a working screen | M1 import loop and M2 execution loop are visible checkpoints; no speculative packages/services |

M0–M3 do not require another product decision. R3 now has a supplier milestone (Rascal M10);
its shared profile still needs agreement because independent Operations is core to the full release.
C2/C3 need named owner delivery checkpoints too; R2 remains optional. Do not imply an Actions-only
development build satisfies those release requirements. Product choices remain Q1–Q19.
