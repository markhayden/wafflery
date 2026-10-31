# Wafflehouse

A local browser workspace for trying visual pipeline tools, inspecting their behavior, and
taking a tested configuration into your own code. The first release focuses on Rascal and
Squint independently. Pixel, cross-tool workflows, and batch evaluations follow later.

This directory contains the approved plan, an interactive mock, and the M1 Rust/React application.
The application starts locally, authenticates browser tabs and stores imported images across
restarts. It does not yet run tools or store runs. Mock installation, provider calls, run
outputs, costs, and persistence remain simulated; a mock manifest is not a runnable bundle.

## Develop the foundation

From this directory, with Rust 1.99.1, Bun 1.4.3 and Node 22.13.1:

```sh
bun install --cwd ui --frozen-lockfile
bun run --cwd ui dev
```

The UI opens at `http://127.0.0.1:5173`. To develop the shared component in Storybook,
run `bun run --cwd ui storybook` and open `http://127.0.0.1:6006`.
The M1 host serves the embedded UI. Run `cargo run -- --foreground --no-open` to keep
it attached to the terminal, or `cargo run -- --no-open` to start/attach to a detached host
and print its local address. Omit `--no-open` to open the browser. `--tool rascal` selects a
workspace context; registration and execution are still being built. Development/tests can
isolate their data with `--data-dir <path>`. Closing a browser tab does not quit the host.
An address printed with `--no-open` contains no authentication secret; run the launcher again
without that flag to open an authenticated tab. Images live under
`~/Library/Application Support/Wafflehouse/Library` by default. The development foreground host
stops with Ctrl-C; an idle detached host supports authenticated Quit via its API, with the visible
Quit control arriving alongside active-run handling.

```sh
bun run --cwd ui typecheck
bun run --cwd ui test:unit
bun test scripts/ui-manifest.test.ts
bun run --cwd ui build:storybook
bun run scripts/build-ui.ts
bun run scripts/check-ui.ts
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --locked --release
```

Rust builds use `ui/prebuilt` without invoking JavaScript. Regenerate it after changing UI
inputs; debug builds warn when stale, release/CI builds fail. `check-ui.ts` verifies the manifest
and compares two clean builds. Keep prebuilt assets and their manifest in version control.

For browser checks, first install the matching Chromium/WebKit builds with
`cd ui && bunx playwright install chromium webkit`, then from the project root run
`bun run --cwd ui test:browser`. Tests own ports 4173 and 6006 and do not reuse existing servers.
See the [M0 checkpoint](docs/reviews/2026-10-31-m0-checkpoint.md) and
[M1 checkpoint](docs/reviews/2026-10-31-m1-checkpoint.md) for verification evidence. M2–M3 are pending.

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
