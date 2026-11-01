# ADR-0001: React UI with shared components and Storybook

Status: Accepted direction; dependency versions and implementation plan remain open.
Date: 2026-10-30

## Context

Wafflery needs a refined, interactive local workspace, with reusable forms, image/artifact
inspection, history, and debugging. The selected Focused picker design already requires task
and operation selection to share one component. Installation should remain one local host,
and building the Rust host must not require a JavaScript toolchain.

## Decision

Use React and TypeScript with Vite for the browser UI, served by the Rust host. Build a small
Wafflery component library using selected shadcn components as a starting point and their
underlying interaction primitives. Own the product styling and compose specialized image and
artifact components as needed. Use Storybook with React/Vite from the first UI implementation.

Define shared design tokens and stories for meaningful component states and responsive cases.
Add components to satisfy actual screens, checking existing patterns before adding a new one.
Task and Operation selectors share one component. Storybook makes duplication visible; review
and component ownership are still needed to prevent bloat.

The shared drawer and image comparison viewer are also reusable product components. Drawer
stories cover export, run details, browsing/saving examples, long content, narrow screens, focus
return, and reduced motion. Image comparison stories cover source/result, mask overlays,
baseline/attempt, keyboard divider control, and incompatible image frames. Tool-specific screens
must not fork these components to change their basic dimensions or interaction behavior.

## Packaging

Ship built UI assets embedded in the executable. Include prebuilt assets in source distributions
so cargo builds do not invoke a frontend build. UI development and asset regeneration require
the JavaScript toolchain; ordinary use does not. Storybook is development-only and excluded
from the installed application. Asset freshness verification needs definition in the build plan.

## Alternatives and consequences

Next.js adds framework responsibilities that are unnecessary for this browser client with a
Rust backend. Vanilla JavaScript remains appropriate for disposable mocks, but shared state and
component reuse favor React for the real application. Building all controls from scratch would
also make us responsible for interaction and accessibility behavior already handled by existing
primitives. Selective reuse still requires dependency maintenance and browser verification.

This decision covers the UI foundation; discovery, tool capabilities, typed artifact handling,
report embedding, and reproducible invocation export still need explicit backend contracts.

## References

- [shadcn ownership and composition](https://ui.shadcn.com/docs)
- [Storybook React/Vite integration](https://storybook.js.org/docs/get-started/frameworks/react-vite)
- [Vite production assets](https://vite.dev/guide/build)
