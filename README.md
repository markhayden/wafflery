# Wafflehouse

A local browser workspace for trying visual pipeline tools, inspecting their behavior, and
taking a tested configuration into your own code. The first release focuses on Rascal and
Squint independently. Pixel, cross-tool workflows, and batch evaluations follow later.

This directory contains the Rust/React development application, its reviewed plans, and an earlier
interactive mock. The application runs real offline MCP actions, preserves evidence, reopens
history, compares baselines, saves examples and exports recorded calls/evidence. Production tool
installation, paid runs, native replay and tool-owned reports remain later integration work.

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

**Clean up library** previews selected runs/examples and reclaimable image bytes. Shared examples
retain their images; deleting a baseline run clears its pin. Cleanup requires idle tools, rechecks
references, and never deletes originals or opaque tool workspaces. SQLite metadata/backups and
unconfirmed execution copies are additional storage, outside the image-byte preview. Known
disposable inputs are removed after preservation or confirmed process exit. Crash leftovers whose
owner cannot be proven stopped are retained. There is no automatic expiration.

Closing a tab leaves the host running. **Quit workspace** asks before cancelling active work,
waits for evidence and owned tools, and offers a separate force action if they do not stop. A
cancelled call without a structured run identity remains unconfirmed until its process stops;
it is never automatically retried.

The development script uses a temporary directory, which macOS may eventually remove. Ordinary
launches use `~/Library/Application Support/Wafflehouse/Library` for durable personal storage.
To run that host use `cargo run --`; to isolate it use `--data-dir <path>`. `--foreground --no-open`
keeps it attached to the terminal, and Ctrl-C requests orderly shutdown. Registered tool descriptors
are currently explicit development configuration; see [the test kit](testkit/README.md), including
the opt-in real Rascal offline smoke. Automatic discovery and installation remain M4 work.

## Develop and verify

### Try real Rascal

With a stable contract 0.1.0 Rascal binary already built or installed, select its absolute path:

```sh
bun run scripts/dev-rascal.ts /absolute/path/to/rascal
```

This creates an isolated library and Rascal workspace, registers the executable explicitly,
then opens Wafflehouse through the actual `rascal playground` launcher. It supplies no provider
configuration or credentials. Choose **Heal a seam**, upload an image, and set both **image
coordinates** and **projection** to equirectangular to exercise the wrap-edge mask. The owned
`ui/tests/fixtures/panorama.png` provides a suitable test image. Without a
filler this produces mask evidence, not an AI repair. Required location actions remain unavailable
until location binding is implemented; prompt-based actions may require unconfigured dependencies.

The script prints the exact command to reopen the same library. Add `--no-open` to start through
the direct host entry without opening a browser. Neither mode installs software. This is a
temporary development library; the normal launcher uses the durable personal library.

### Register an installed tool explicitly

The host accepts a JSON array of descriptors with `id`, absolute `executable`, `args` (an array,
never a shell command), absolute `cwd`, and absolute directory `roots`. See the real-tool helper
for a concrete Rascal descriptor. These records select programs that will execute when the host
starts; only register executables you intend to run. The host supplies its own input staging root
and starts the tool with an empty environment. Provider setup comes in a later integration slice.

While the host is stopped:

```sh
target/debug/wafflehouse --register /absolute/path/to/tools.json
```

Registration validates and atomically appends records without executing them. Repeating an
identical record is harmless; conflicting IDs are refused. It cannot change a live host's
configuration. `--data-dir <path>` selects a separate library parent. Alternatively,
`WAFFLEHOUSE_DATA_DIR=/absolute/path` selects that same parent for both `wafflehouse` and
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
cargo build --locked --release --bin wafflehouse
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

See the [M0](docs/reviews/2026-10-31-m0-checkpoint.md),
[M1](docs/reviews/2026-10-31-m1-checkpoint.md),
[M2](docs/reviews/2026-10-31-m2-checkpoint.md), and
[M3](docs/reviews/2026-10-31-m3-checkpoint.md) checkpoints for exact verification and remaining gates.

## Review the interface

Open [work/mockups.html](work/mockups.html) directly in a browser. On macOS:

```sh
open work/mockups.html
```

Use **Start from** to review Wafflehouse-first, Rascal-first, Squint-first, or a ready workspace.
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
- [Spec review disposition](docs/reviews/2026-10-30-spec-review-disposition.md): Claude's findings,
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
