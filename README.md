# Wafflehouse

A local browser workspace for trying visual pipeline tools, inspecting their behavior, and
taking a tested configuration into your own code. The first release focuses on Rascal and
Squint independently. Pixel, cross-tool workflows, and batch evaluations follow later.

This directory contains the approved plan, an interactive mock, and the M0 Rust/React scaffold.
The application does not yet run tools or store runs. Mock installation, provider calls, run
outputs, costs, and persistence remain simulated; a mock manifest is not a runnable bundle.

## Develop the foundation

From this directory, with Rust 1.99.1, Bun 1.4.3 and Node 22.13.1:

```sh
bun install --cwd ui --frozen-lockfile
bun run --cwd ui dev
```

The UI opens at `http://127.0.0.1:5173`. To develop the shared component in Storybook,
run `bun run --cwd ui storybook` and open `http://127.0.0.1:6006`.
The Rust executable currently supports help/version only; the HTTP host is M1.

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
See the [M0 checkpoint](docs/reviews/2026-10-31-m0-checkpoint.md) for passed checks and environment
blocks. M0 is not yet signed off; no M1–M3 completion is claimed.

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
  developer commands. M0 commands above are implemented; host commands remain planned.
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
