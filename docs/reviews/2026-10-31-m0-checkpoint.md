# M0 implementation checkpoint

Owner approved M0–M3 on 2026-10-31. Permissions were subsequently enabled: Git initialization
and the advisory audit now work. M0 is complete. M1–M3 implementation continues next. No paid calls, provider setup, sibling changes, publishing or tool installation occurred.

## Task status

| Slice | Evidence / remaining work |
|---|---|
| F01a baseline | Ignore rules prepared. git init -b main fails with Operation not permitted on .git. Restriction subsequently lifted: repository initialized and documents committed as 87d1457. No remote created. |
| F01b Rust package | Pinned toolchain, Cargo.lock, help/version scaffold, build, format and Clippy pass. CLI smoke checks cover help/version, no arguments and usage exit 64. |
| F02a UI graph | Exact versions installed; frozen install, strict source typecheck and production build pass. Initial advisory request was blocked. After permissions changed, patched Storybook and transitive glob/minimatch; audit now reports no vulnerabilities. |
| F02b tokens/config | Reviewed mock palette/spacing, minimal honest empty state, loopback-only Vite configuration. Desktop and 320px screenshots inspected in real browsers. |
| F03a component library | Shared Button used in the app, default/disabled/busy/long-label stories; static Storybook builds. Keyboard check passes in Chromium and WebKit. |
| F03b test entry points | Unit and Chromium/WebKit browser scripts/fixtures exist. Three unit behavior tests pass. Initial localhost EPERM resolved by permissions change; all 12 browser checks now pass. |
| F04a reproducible UI | Manifest tests detect changed/deleted/added bytes and reject symlinks. Two clean Vite builds match the prebuilt hashes byte-for-byte. |
| F04b Rust embedding | Two Rust integration tests pass. Build-script process probes verify fresh debug, stale-debug warning, stale strict/release/CI rejection and missing-file rejection. Release compiles with Node/Bun absent from PATH. |

Commits: 87d1457 (reviewed documents), 4ff8e4a (UI, Storybook and deterministic builds),
fec5cc0 (Rust embedding). The scaffold was authored while Git was blocked; recovery commits
group its coupled UI/build files and Rust files rather than manufacturing earlier task snapshots.
Future slices use the planned small commits. No alternate Git directory was used.

## Verification and environment

- Rust/Cargo 1.99.1, Bun 1.4.3, Node 22.13.1 via Volta.
- React/DOM 19.3.1, Vite 8.3.3, TypeScript 6.0.3, Storybook/react-vite 10.6.5,
  Playwright 1.64.1. Direct supporting types are pinned in ui/package.json.
- 3 UI unit tests, 2 manifest tests, 2 Rust integration tests and 12 real browser tests pass.
  Chromium and WebKit cover 320/768/1440px layouts, button keyboard focus, disabled busy state
  and long-label wrapping. Console errors are checked. Desktop/narrow screenshots were inspected.
  WebKit’s initial Tab assertion failed; macOS Safari uses Option-Tab for all controls, which
  passes. No global keyboard preference was changed. A full two-browser rerun passes.
  Native Safari app verification remains an M3 check, distinct from Playwright WebKit.
- Playwright needs Chromium/headless revision 1248 and WebKit 2370. Both were downloaded successfully after permissions changed; older cached builds were not substituted.
- The initial audit was blocked. After permission changes, bun audit reported two Storybook
  advisories and two transitive matching-library advisories. Storybook 10.6.5 resolves the first
  pair; exact overrides glob 14.0.2 and minimatch 11.0.1 resolve the second. Reviewed the docgen
  plugin’s globSync call (absolute/cwd/ignore/nodir) and verified its Storybook build. No peer
  constraint is ignored. Remove these overrides when upstream ranges include patched versions.
  The subsequent audit found no vulnerabilities in 259 packages.
- A fresh release build without JS tools on PATH passes without warnings through the rustup
  launcher. The earlier direct-toolchain-binary probe bypassed its library-path setup and caused
  an objcopy/libLLVM warning; no global toolchain changes were necessary.
- Storybook's development bundle emits a size warning; it is not embedded in the application.

## Type-checking decision

Storybook's declaration files produce generic-constraint and unused ts-expect-error failures
under TypeScript 6.0.3. Reproduced with 10.6.0 and 10.6.5 and with exactOptionalPropertyTypes
disabled; the errors are in dependency declarations, not authored code. Enabled skipLibCheck; the subsequent security audit required the 10.6.5 patch release. Strict checks, exact optional properties and indexed
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

## Next

Continue F05 with the real loopback host and ownership tests. M0 has no remaining environment
block. No remotes or published releases were created.
