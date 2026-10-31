# M0 implementation checkpoint

Owner approved M0–M3 on 2026-10-31. M0 scaffold is implemented; milestone completion is blocked
on Git creation, real-browser verification and the dependency advisory check. M1–M3 have not
started. No paid calls, provider setup, sibling changes, publishing or tool installation occurred.

## Task status

| Slice | Evidence / remaining work |
|---|---|
| F01a baseline | Ignore rules prepared. git init -b main fails with Operation not permitted on .git. No repository, commits, branch or remote created. |
| F01b Rust package | Pinned toolchain, Cargo.lock, help/version scaffold, build, format and Clippy pass. CLI smoke checks cover help/version, no arguments and usage exit 64. |
| F02a UI graph | Exact versions installed; frozen install, strict source typecheck and production build pass. Advisory audit blocked by DNSResolveFailed. |
| F02b tokens/config | Reviewed mock palette/spacing, minimal honest empty state, loopback-only Vite configuration. Visual verification pending. |
| F03a component library | Shared Button used in the app, default/disabled/busy/long-label stories; static Storybook builds. Runtime keyboard check pending. |
| F03b test entry points | Unit and Chromium/WebKit browser scripts/fixtures exist. Three unit behavior tests pass. Browser suite stops before tests: localhost listen EPERM. |
| F04a reproducible UI | Manifest tests detect changed/deleted/added bytes and reject symlinks. Two clean Vite builds match the prebuilt hashes byte-for-byte. |
| F04b Rust embedding | Two Rust integration tests pass. Build-script process probes verify fresh debug, stale-debug warning, stale strict/release/CI rejection and missing-file rejection. Release compiles with Node/Bun absent from PATH. |

No task is recorded as committed. Files remain available for the planned commit groups after
Git access is restored; no alternate Git directory was used to evade the restriction.

## Verification and environment

- Rust/Cargo 1.99.1, Bun 1.4.3, Node 22.13.1 via Volta.
- React/DOM 19.3.1, Vite 8.3.3, TypeScript 6.0.3, Storybook/react-vite 10.6.0,
  Playwright 1.64.1. Direct supporting types are pinned in ui/package.json.
- 3 UI unit tests, 2 manifest tests and 2 Rust integration tests pass. No browser test passed
  or failed assertions: its server could not start. No visual/keyboard/Safari verification claimed.
- Playwright needs Chromium/headless revision 1248 and WebKit 2370. Existing cached builds
  are older (Chromium 1223/1243 and WebKit 2287); they do not satisfy this version.
- Normal Bun installation succeeded despite direct curl/audit DNS restrictions. bun audit failed
  contacting the advisory endpoint; this is not a clean security audit.
- Release build succeeded without JS tools on PATH. The installed Rust toolchain emitted a
  rust-objcopy/libLLVM stripping warning. No global toolchain files were changed; distribution
  checks must address that warning before treating this as a packaged release.
- Storybook's development bundle emits a size warning; it is not embedded in the application.

## Type-checking decision

Storybook's declaration files produce generic-constraint and unused ts-expect-error failures
under TypeScript 6.0.3. Reproduced with 10.6.0 and 10.6.5 and with exactOptionalPropertyTypes
disabled; the errors are in dependency declarations, not authored code. Retained the planned
10.6.0 versions and enabled skipLibCheck. Strict checks, exact optional properties and indexed
access checks remain enabled for app/stories/tests; build scripts are also typechecked.
No dependency peer constraint is ignored or package source patched. This deliberately forgoes
checking internal .d.ts consistency; it is not evidence those declarations are error-free.
See [TypeScript's documented scope](https://www.typescriptlang.org/tsconfig/skipLibCheck.html).

## Author review

Reviewed correctness, scope, boundary handling and generated-file ownership. A freshness test
initially depended on the working UI being fresh; replaced it with an isolated fixture so normal
debug UI edits do not make cargo test fail. Browser tests serve built output rather than HMR
pages. No speculative host/library/tool modules were created beyond the asset module consumed
by tests. No secrets or runtime library data are included in source/generated output.

Patterns checked against [React createRoot](https://react.dev/reference/react-dom/client/createRoot),
[Vite server options](https://vite.dev/config/server-options),
[Storybook React/Vite](https://storybook.js.org/docs/get-started/frameworks/react-vite), and
[Playwright web servers](https://playwright.dev/docs/test-webserver). SHA-256 API usage was
checked against the installed sha2 0.11.1 source.

## Resume

Restore repository metadata writes and localhost server/browser execution in the working session.
Install the matching Playwright browsers, run test:browser (including screenshot inspection),
complete the advisory audit and Git checkpoints. Do not mark M0 complete just because static
builds pass. Then continue F05 with the real loopback host and ownership tests.
