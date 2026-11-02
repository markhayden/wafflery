# Wafflery

Formerly Wafflehouse.

A local browser workspace for trying visual pipeline tools, inspecting their behavior, and
taking a tested configuration into your own code. The first release focuses on Rascal and
Squint independently. Pixel, cross-tool workflows, and batch evaluations follow later.

This directory contains the Rust/React development application, its reviewed plans, and an earlier
interactive mock. The application runs real offline MCP actions, preserves evidence, reopens
history, compares baselines, saves examples and exports recorded calls/evidence. Production tool
installation and native replay remain later integration work; saved runs can open declared tool reports.
The authenticated prepared-call API can execute paid actions only from a current owned estimate;
its automated verification uses synthetic spend, without provider charges.

## Try the working application

From this directory, with Rust 1.99.1, Bun 1.4.3 and Node 22.13.1:

```sh
bun install --cwd ui --frozen-lockfile
bun run scripts/dev-fake.ts
```

This builds the embedded UI and independent `sample-lab` tool, creates an isolated development
library and opens an authenticated browser tab. The tool adds an illustrative band to an image;
it does not repair images. It requires no provider keys and makes no paid calls. The script prints
its data directory and the command to reopen that same library. `--no-open` starts it without
opening a browser; the printed address contains no authentication secret.

Choose a PNG/JPEG, enter a label, and run. Inspect saved inputs/raw result/recorded MCP/journal in
the shared drawer, drag the before/after divider, pin a baseline, or save an example. Opening an
example fills a new draft without executing it. History and examples are separate. The pipeline
drawer copies the saved request/setup and downloads a `.tar.gz` evidence package, optionally
including image bytes. It does not claim a portable native replay bundle or generate unverified CLI
flags. Editing the draft never changes a saved run.

**Inspect run → Reports** lists the views declared for that saved run. Choose a view and use
**Open report** to start it; selecting one alone does not run anything. Server viewers and
standalone documents share the drawer, with diagnostics available if loading fails. Switching
back to evidence or closing inspection releases the viewer. Inactive viewers expire; a process
that will not close offers a separately confirmed **Force stop report**. Reports do not change
the saved run's outcome or become part of its evidence archive.

**Tool setup** opens the shared drawer with the selected action's declared credential requirements
and live session spending: limit, remaining allowance, reported spend and active reservations.
Refresh reads the running tool without executing an action or resetting its budget. Credential
names do not prove a key is configured or valid. **Restart tool session** requires confirmation
and an idle tool; it reprobes the selected config and starts a new spending ledger. Earlier runs
and unknown charges remain in history. The drawer reports the new session ID, and other tabs
refresh their setup observations. A failed restart leaves recovery available in the same drawer.
Environment changes require quitting and relaunching Wafflery from the updated terminal.
**Folder access** separates requested input folders from folders Wafflery may read results from.
The current v0.3 session cannot report its effective input permissions, so setup says **Not reported**;
it never treats the requested list as proof of access. Failed session refreshes show **Unavailable**.
Changing registration folders requires stopping Wafflery; changing the tool's configuration requires
restarting that tool session. Effective v0.4 permissions are added in M5.03.
Paid actions expose explicit cap and estimate controls. Edits, declaration/session changes and
expiry invalidate the quote. **Run** sends the exact reviewed preparation; another paid attempt
requires a fresh estimate. If acknowledgement is lost, **Retry submission** keeps the original run
ID and inputs while preserving later edits separately. There is no automatic paid retry.
The result summary explains stale estimates, credential rejection, caps and session budgets.
Failed and cancelled runs retain their reported spend; unresolved charges are shown as “at most,”
not zero. **Review tool setup** opens setup for that saved run’s tool and action.

**Clean up library** previews selected runs/examples and reclaimable image bytes. Shared examples
retain their images; deleting a baseline run clears its pin. Cleanup requires idle tools, rechecks
references, and never deletes originals or opaque tool workspaces. SQLite metadata/backups and
unconfirmed execution copies are additional storage, outside the image-byte preview. Known
disposable inputs are removed after preservation or confirmed process exit. Crash leftovers whose
owner cannot be proven stopped are retained. There is no automatic expiration.

Closing a tab leaves the host running. The **Quit** power icon opens a confirmation before cancelling active work,
waits for evidence and owned tools, and offers a separate force action if they do not stop. A
cancelled call without a structured run identity remains unconfirmed until its process stops;
it is never automatically retried.

The development script uses a temporary directory, which macOS may eventually remove. Ordinary
launches use `~/Library/Application Support/Wafflery/Library` for durable personal storage.
To run that host use `cargo run --`; to isolate it use `--data-dir <path>`. `--foreground --no-open`
keeps it attached to the terminal, and Ctrl-C requests orderly shutdown. Registered tool descriptors
are currently explicit development configuration; see [the test kit](testkit/README.md), including
the opt-in real Rascal offline smoke. Automatic discovery and installation remain M4 work.

## Develop and verify

[CI](.github/workflows/ci.yml) runs the full PLAN matrix on `macos-15` for pull requests,
pushes to main and manual dispatch. Action revisions and runtime versions are pinned; Cargo
and Bun dependencies are cached, and matching Chromium/WebKit browsers are installed explicitly.
CI uses the independent offline peer without provider keys. Local and hosted builds resolve
exactly table-manners 0.3.0 from crates.io using Cargo.lock; no sibling contract checkout is
required. The three Rust and four browser real-tool probes remain explicit opt-ins. Checked-in UI assets are verified
before any build can regenerate them. Failure diagnostics include Playwright traces, screenshots and an allowlisted attempt summary.
See the [CI checkpoint](docs/reviews/2026-11-01-ci.md) for local versus hosted verification.

### Try real Rascal

The v0.3 client cutover is being verified against the independent peer. Real Rascal M12 and
Squint M9 integration is gated by V18; the previous v0.2 binaries are now rejected.
With a stable contract 0.3.0 Rascal binary already built or installed, select its absolute path:

The tool's `playground` launcher must also implement the renamed H1 interface (`wafflery` and
`WAFFLERY_BIN`). Older contract-compatible binaries still work for direct MCP tests, but must not
be used with these launcher helpers. The same requirement applies to the Squint helper below.

```sh
bun run scripts/dev-rascal.ts /absolute/path/to/rascal
```

This creates an isolated library and Rascal workspace, registers the executable explicitly,
then opens Wafflery through the actual `rascal playground` launcher. It supplies no provider
configuration or credentials. Choose **Heal a seam**, upload an image, and set both **image
coordinates** and **projection** to equirectangular to exercise the wrap-edge mask. The owned
`ui/tests/fixtures/panorama.png` provides a suitable test image. Without a
filler this produces mask evidence, not an AI repair. Required location actions remain unavailable
until location binding is implemented; prompt-based actions may require unconfigured dependencies.

The script prints the exact command to reopen the same library. Add `--no-open` to start through
the direct host entry without opening a browser. Neither mode installs software. This is a
temporary development library; the normal launcher uses the durable personal library.

### Register an installed tool explicitly

For an isolated **two-tool offline test** workspace:

```sh
bun run scripts/dev-pipeline.ts /absolute/path/to/rascal /absolute/path/to/testkit/squint
```

Use stable contract 0.3.0 executables; Squint needs the `testkit` feature and its v0.3 M9 implementation. The helper registers both tools and opens through `squint playground`. Select
**Source fidelity** and provide candidate/source images. Squint returns a **scripted fixture
verdict, not a model judgment**; Rascal runs without a filler. No credentials are supplied.
The helper prints a reopen command. The estimate journal follow-up is resolved and the probe now
preserves complete evidence. No existing library is modified.

The host executes exactly table-manners 0.3.0. Earlier saved history remains readable.
**Inspect run → Session** shows the server ledger before/after a run and any structured run
association. This is a server-session ledger, not a provider account balance. Cancellation
can recover a terminal result from its associated resource; missing or contradictory evidence
remains unconfirmed. Paid dispatch remains disabled pending provider readiness and per-call caps.

### Registration format

The host accepts a JSON array of descriptors with `id`, absolute `executable`, `args` (an array,
never a shell command), absolute `cwd`, required `allow_dirs` and `evidence_dirs` arrays, and an optional absolute
`config` path. The host appends `--config` and that path as separate arguments and never reads
the file; a conflicting `--config` in `args` is refused. Omit `config` for the tool’s default lookup. See the real-tool helper
for a concrete Rascal descriptor. These records select programs that will execute when the host
starts; only register executables you intend to run. Both directory arrays contain existing absolute
directories, resolved canonically during validation. Directory names must remain valid UTF-8 after
resolving symlinks so permissions can be represented exactly. `allow_dirs` authorizes caller inputs; the host
passes each as a literal `--allow-dir` argument. Hand-written `--allow-dir` flags in `args` are refused.
`evidence_dirs` authorizes Wafflery to read returned image paths and run/report directories; it is
never passed to the child. Neither list grants the other permission. Empty lists are allowed.
The host supplies only its own `Library/execution` input staging directory automatically
and starts the tool with a minimal environment for default configuration lookup and locale,
plus a fixed system PATH. See the [environment policy](ENGINEERING.md#child-process-environment).
The host first probes the tool over MCP with that base environment, closes the probe, then starts
one serving session with only its declared credential and non-secret environment variables.
Missing variables remain absent; the tool reports its readiness. If serving adds requirement
names absent from the probe, setup is marked incomplete without restarting automatically.
Configuration contents and environment values are never copied into the host library.

The former `roots` registration field is rejected. Before adopting this build, quit the old host,
save the old binary and move its `tools.json` to a separate backup (do not overwrite an existing
backup). Prepare a new descriptor file with both explicit arrays, selecting input and evidence
permissions separately, then register it with the command below. Remove old hand-written
`--allow-dir` arguments. No real registrations or stored history are migrated automatically.
Rollback requires the old binary, matching peers and the original backed-up registration together;
do not restore old grants under the new binary. Production contract validation remains 0.3.0 until
M5.03, so this intermediate build needs peers that accept the explicit grant flags. The earlier
Squint snapshot `54b1f41` uses MCP roots and is unsupported by this intermediate build; use the
approved v0.4 Squint `6dd1e3a` only after the M5.03 validator cutover. The prior V18 interoperability
results remain historical evidence, not a claim that those snapshots pass this registration format.

While the host is stopped:

```sh
target/debug/wafflery --register /absolute/path/to/tools.json
```

Registration validates and atomically appends records without executing them. Repeating an
identical record is harmless; conflicting IDs are refused. To change an existing tool's config
path or other registration fields, quit the host and run
`wafflery --register <file> --replace-registration`. Replacement only accepts existing IDs and applies the entire file
atomically; an unknown ID leaves all registrations unchanged. It cannot change a live host's
configuration. `--data-dir <path>` selects a separate library parent. Alternatively,
`WAFFLERY_DATA_DIR=/absolute/path` selects that same parent for both `wafflery` and
`rascal playground`; an explicit `--data-dir` takes precedence. The environment selection does
not alter the tool's launcher arguments. Automatic discovery and installation are still pending.

### Development checks

The frontend is React + Vite; the local host is Rust with bundled SQLite. Shared controls have
Storybook stories. JavaScript tooling is needed to develop the UI, not to run a built binary.

```sh
bun run --cwd ui dev
bun run --cwd ui storybook
bun run --cwd ui typecheck
bun run --cwd ui test:unit
bun test scripts/ui-manifest.test.ts
bun run --cwd ui build:storybook
bun run scripts/build-ui.ts
bun run scripts/check-ui.ts
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --locked --release --bin wafflery
```

Vite opens at `http://127.0.0.1:5173`; Storybook at `http://127.0.0.1:6006`.
The connected development flow uses the host’s embedded UI through `dev-fake.ts`.
Rust builds use `ui/prebuilt` without invoking JavaScript. Regenerate it after changing UI
inputs; debug builds warn when stale, release/CI builds fail. `check-ui.ts` verifies the manifest
and compares two clean builds. Keep prebuilt assets and their manifest in version control.

For browser checks, first install the matching Chromium/WebKit builds with
`cd ui && bunx playwright install chromium webkit`, then from the project root run
`bun run --cwd ui test:browser`. Tests own ports 4173 and 6006 and do not reuse existing servers.
They use disposable libraries, actual MCP processes, and no paid providers. Native Safari is a
separate manual check, not an assertion based on automated WebKit.

`test:browser` builds the debug host and independent peer once before Playwright starts its
parallel workers. Fixtures never invoke Cargo. Playwright checks both executables before tests
and reports the build command if either is missing or unusable; direct `playwright test` runs
require those binaries to have been built already.

See the [M0](docs/reviews/2026-10-31-m0-checkpoint.md),
[M1](docs/reviews/2026-10-31-m1-checkpoint.md),
[M2](docs/reviews/2026-10-31-m2-checkpoint.md), and
[M3](docs/reviews/2026-10-31-m3-checkpoint.md) checkpoints for exact verification and remaining gates.

## Review the interface

Open [work/mockups.html](work/mockups.html) directly in a browser. On macOS:

```sh
open work/mockups.html
```

Use **Start from** to review Wafflery-first, Rascal-first, Squint-first, or a ready workspace.
Try a local Line operation, change its parameters, pin a baseline, compare attempts, save an
example, and open Use in your pipeline. Tool-first views begin after terminal installation and
host startup. Changing the starting point resets demo history and saved examples.

To rebuild the self-contained preview after editing its source components:

```sh
python3 work/build-mockups.py
```

The mock keeps editable markup/data in `work/activity-hierarchy.html`, interaction controllers
and shared view fragments in `work/`, and the preview wrapper in `work/preview-controls.html`.
The build script synchronizes the assembled source and standalone preview. Browser-native
dialog, focus, range, and layout behavior still require real-browser verification; DOM checks
alone do not establish those properties.

## Product and implementation documents

- [SPEC.md](SPEC.md): objective, accepted decisions, engineering work still to resolve.
- [PLAN.md](PLAN.md): reviewable foundation milestones, checks, commits and later feature gates.
- [Plan review disposition](docs/reviews/2026-10-31-plan-review-disposition.md): review findings, corrections and remaining coordination.
- [Integration source inventory](docs/integration-sources.md): current sibling revisions and verified capabilities.
- [H1 launcher proposal](docs/handoffs/H1-launcher-proposal.md) and [C4 contract proposal](docs/handoffs/C4-v0.2-proposal.md): ready for owner/schema review.
- [Foundation runtime](docs/design/foundation-runtime.md): selected stack, boundaries and planned
  developer commands. The host and image-library commands above are implemented.
- [ENGINEERING.md](ENGINEERING.md): proposed storage, process ownership, lifecycle, and boundaries.
- [Spec review disposition](docs/reviews/2026-10-30-spec-review-disposition.md): review findings,
  corrections, and remaining integration limits.
- [ACCEPTANCE.md](ACCEPTANCE.md): testable first-release outcomes and planning gates.
- [USER-JOURNEYS.md](USER-JOURNEYS.md): installation, experimentation, preservation, and handoff.
- [INTERFACE.md](INTERFACE.md): interface and tool-capability context.
- [TOOL-REQUIREMENTS.md](TOOL-REQUIREMENTS.md): owner packets for table-manners, Rascal, and Squint.
- [UI foundation ADR](docs/decisions/0001-ui-foundation.md): React/TypeScript/Vite, selected shadcn
  foundations, Storybook, and a Rust host with embedded built UI assets.
- [BRIEF.md](BRIEF.md): original input; later decisions and source audits supersede stale details.

The first release targets macOS; Linux is a possible low-priority follow-up. The proposed
[local library and host design](docs/decisions/0002-local-library-and-host.md) implements the
accepted single-library experience without automatic job recovery.

M0–M3 implementation was approved on 2026-10-31. The plan retains verification and commit
checkpoints; actual macOS/browser support remains subject to runtime verification.
