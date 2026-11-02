# Foundation runtime and developer workflow

Status: M0–M3 approved, 2026-10-31. M0 complete with verification evidence recorded in the
[checkpoint](../reviews/2026-10-31-m0-checkpoint.md); later host behavior remains planned.
[Current sibling/environment evidence](../integration-sources.md) supersedes
the earlier audit snapshots. This scopes [ENGINEERING.md](../../ENGINEERING.md) into independently
buildable work; it does not replace later tool-contract gates.

## Selected stack

One Cargo application package, `wafflery`, with a library for testable host modules and a
small executable entry point. One React UI package under `ui/`. No additional application
framework, ORM, database service, global frontend state library, or plugin SDK initially.

| Dependency | Foundation selection | Evidence / purpose |
|---|---|---|
| Rust | Toolchain 1.99.1, edition 2024 | Installed; agrees with the tools' 1.99 baseline |
| Tokio | 1.54.1 | Existing Rascal lockfile; process/network/async runtime |
| Axum | 0.8.9 | [Official reference](https://docs.rs/axum/latest/axum/); HTTP routing |
| rusqlite | 0.41.1, bundled SQLite | [Official project](https://github.com/rusqlite/rusqlite#optional-features); no system SQLite dependency |
| rmcp | Exactly 3.8.0 | [SDK transports](https://docs.rs/rmcp/latest/rmcp/transport/index.html); client + async read/write transport |
| table-manners | Exactly 0.3.0 | Published crates.io =0.3.0; source/checksum tracked in the [inventory](../integration-sources.md) |
| Bun | 1.4.3 | Installed; package installation, scripts, pure TypeScript tests |
| Node | Record resolved executable in M0 | Agent shell: 22.13.1 via Volta; owner review: 24.18.0. Both observations are retained; use one tested developer version for repeatable builds, not a production runtime |
| React / React DOM | 19.3.1, matched versions | [Registry release](https://registry.npmjs.org/react/latest); installed compatibility still to verify |
| Vite | 8.3.3 | [Official release](https://github.com/vitejs/vite/releases/tag/v8.3.3) |
| TypeScript | 6.0.3 | [Official release](https://github.com/microsoft/TypeScript/releases/tag/v6.0.3); use established compiler API line for Storybook tooling |
| Storybook / React-Vite | 10.6.5, matched versions | [Release](https://github.com/storybookjs/storybook/releases/tag/v10.6.5), [framework integration](https://storybook.js.org/docs/get-started/frameworks/react-vite) |
| Playwright Test | 1.64.1 | [Registry release](https://registry.npmjs.org/@playwright/test/latest); browser and API integration tests |

These are selected starting versions, not a claim all form a tested dependency graph. Resolve
and pin supporting packages in M0: React types, Vite React plugin, and only the UI primitives
actually used. Use exact direct dependency versions, Cargo.lock, and bun.lock. No floating
`latest` commands in checked-in scripts. Check peer/engine compatibility and advisories before
accepting the locks. If a selected combination fails, make a documented patch-level adjustment;
do not work around it with ignored peer errors. Shell registry access is currently restricted,
so this planning pass verified releases through official web sources and existing local caches,
not an installation. [Bun documents committed lockfiles](https://bun.sh/docs/pm/lockfile).

Reuse verified tool dependency versions where needed: serde 1.0.229, serde_json 1.0.154,
thiserror 2.0.21, clap 4.6.7, sha2 0.11.1, image 0.25.10 (PNG/JPEG initially), jsonschema
0.60.2, tracing 0.1.44, and tracing-subscriber 0.3.23. Add each only with its first consuming
slice. For F16 evidence archives select tar 0.4.46 and flate2 1.1.10 (versions in the current
Rascal lockfile), added only when that writer is implemented; use explicit safe archive entries.
Use Rust's [File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)
for the host lock. `nix` 0.31.3's safe [setsid](https://docs.rs/nix/latest/nix/unistd/fn.setsid.html)
is the selected macOS detachment mechanism; keep it in the launcher module.

Use owned CSS tokens and CSS modules. Selectively adapt shadcn's
[Radix dialog pattern](https://ui.shadcn.com/docs/components/radix/dialog) and accessible picker
primitives when their components are added; preserve license attribution. Do not install the
entire component catalogue or a utility framework solely to reuse a handful of components.
Use React state/reducers for drafts and a small API client for server state. Storybook documents
real components from the first UI slice. Pure logic uses Bun tests; real interaction uses
Playwright against Storybook and the running host. No second DOM simulator is necessary.

## Repository layout

```text
Cargo.toml, Cargo.lock, rust-toolchain.toml
src/main.rs, src/lib.rs
src/host/                  launch, lock, browser auth, API, event invalidation
src/library/               database, migrations, asset capture/reference cleanup
src/tools/                 MCP client, declared registration, attempt coordination
tests/                     filesystem/process/API integration tests
tests/fixtures/            versioned contract documents and small owned images
testkit/                   independent fake stdio peer (development/test only)
ui/package.json, ui/bun.lock
ui/src/api/                host DTOs, parsing, API calls
ui/src/components/         shared picker, drawer, comparison viewer, fields + stories
ui/src/features/           tool workspace, history, examples
ui/.storybook/
ui/tests/                  pure state/formatting tests and browser tests
ui/prebuilt/               committed release UI assets + input/output digest manifest
scripts/                   dev orchestration, UI generation/freshness verification
docs/                      decisions, designs, owner handoffs, reviews
work/                      reviewed disposable mocks, not a production dependency
```

Create modules when used, not empty placeholder trees. Rust boundaries are modules inside one
package; split crates only when a concrete reusable/test boundary justifies it.

## Storage implementation

Use a single dedicated database worker with one rusqlite connection and bounded messages from
async handlers. Keep short transactions on that worker; do file copies/hashing/decoding on
bounded blocking workers outside transactions. No pool or ORM. Start with rollback journaling,
`synchronous=FULL`, foreign keys enabled, and a five-second busy timeout; read back effective
PRAGMA values in a test. [SQLite's PRAGMA documentation](https://www.sqlite.org/pragma.html)
defines these behaviors. A long file operation must not block reads of run state.

Numbered SQL migrations and a schema-version record are owned by the host. Back up the database
before migration; reject a newer unsupported schema without modifying it. Do not maintain legacy
application APIs, but never throw away saved work to make an older executable start. Code rollback
uses a temporary test library; existing user data is not rolled back automatically.

Publish immutable files before committing their references, as ENGINEERING.md specifies. Verify
byte digest and image dimensions at capture; verify exported bytes against their manifest. Do
not introduce an idle scan service. Imported unused draft assets are visible reclaimable data,
not silently expired history. Database write failure prevents dispatch; result-save failure is
separate from tool outcome. File operations return structured errors, never partial success labels.

## MCP and process ownership

Use an explicit live-execution contract allowlist, currently only `0.3.0`; validate against the
dependency's embedded schemas with the retriever pattern in `table-manners/tests/fixtures.rs`.
Reject every other live version, including 0.1.0 and 0.2.0; preserve older saved evidence without rewriting
or automatically replaying it. The v0.3 cutover is documented in [the execution plan](../plans/v03-execution.md).

Host image bindings are ordered arrays internally. Scalar slots encode one image-in value; v0.3 list slots are declared but their editing/dispatch
implementation waits for V07. No `x-squint` special-case interpretation.

Before dispatch, read and validate the tool's session ledger and persist it with the declaration.
Send the attempt ID as table-manners/client_run_id; record the SDK's actual progressToken at the
transport boundary. Validate the structured run reference against both identities before binding
results. Cancellation can preserve a terminal resource, with its provenance saved separately
from a tool-call response. Bounded paginated lookup never proves an absent run did not execute;
contradictions and exhausted searches remain unconfirmed. No automatic retries or restarts.
The inspector exposes before/after session observations; provider estimates/caps UI and paid
dispatch remain separate work. A changed session ID requires explicit reconnection, not silent
allowance replenishment.

Explicitly select protocol `2025-11-25`; restrict negotiation to that MCP revision. Contract and
MCP versions are separate. M5.02 adopts the agreed explicit `--allow-dir` replacement for MCP
roots while retaining this protocol revision; host evidence-read grants are a separate boundary. The
installed rmcp 3.8.0 source defaults to `2026-07-28` (`src/model.rs`), so SDK defaults are not the
contract. Test the initialization bytes with an independently implemented peer.

Own `tokio::process::Child` and give its stdout/stdin to rmcp's async read/write transport.
Do not use its convenience child wrapper's default shutdown policy: inspected 3.8.0
`transport/child_process.rs` force-kills after three seconds. Wafflery first requests cancel,
awaits a known terminal state where available, then closes stdio and waits. Only explicit force
quit escalates an unresponsive live child. Use process handles, not arbitrary remembered PIDs.
The independent fake must exit on stdio EOF; real parent-death semantics remain an integration
check, not a claim that kill-on-drop survives SIGKILL.

Default `wafflery` launches or attaches and opens the browser. The same binary's internal
host mode starts a new session before the async runtime, with stdio detached from the terminal.
Acquire the OS lock before publishing readiness. Parent and child use a private same-user control
socket in a mode-0700 runtime directory; socket mode 0600. Choose a short runtime path within the
user temporary directory to fit Unix socket limits, recording it privately in host discovery.
Test races and stale sockets; reject unexpected ownership/symlinks. No launchd/login service.

The control socket handles readiness, compatible-host lookup, and new browser bootstrap URLs.
Only the Wafflery binary understands this private protocol. Real tools eventually invoke
`wafflery --tool <id>` under H1; that interface must be agreed before their launcher changes.
F05 verifies the [H1 proposal](../handoffs/H1-launcher-proposal.md), drafted before implementation.
The fake and opt-in real Rascal smoke use explicit executable/argv/root descriptors. No automatic
PATH discovery or installation of real tools occurs in the foundation. Under the tool reservation,
F11a must check the current validated action declares `cost.may_spend: false` before any dispatch;
true/missing/malformed/stale values fail closed. Direct API calls cannot bypass this guard.

## Browser boundary

Bind to `127.0.0.1` on port 0. Authenticate private APIs with per-browser bearer tokens, not
cookies: cookies are not isolated by port, while tool report origins may use another loopback
port. Open a one-time bootstrap secret in the URL fragment, remove it immediately from browser
history, exchange it for a session token, and retain that token only in origin-scoped
sessionStorage. Secret expires after 60 seconds or one exchange; sessions end with the host.
No secrets in query strings, telemetry, exports, or access logs. A reload can reuse its session;
another browser/tab can obtain a fresh bootstrap by rerunning the launcher.

Fetch API and asset data with the Authorization header and `credentials: omit`. Preview verified
images using bounded Blob URLs, revoked when replaced/unmounted. No ambient bearer credentials
go to tool viewer URLs. Validate the exact Host header and reject foreign Origin headers;
all mutations require the expected UI origin and JSON/custom-header request except authenticated
raw uploads. CORS is disabled. Possession of a token is still required when Origin is absent.
Tool execution is trusted same-user code; these checks do not claim protection from a malicious
program already running as the user.

Apply CSP with scripts from self, no object/base/frame embedding initially, no remote resources,
and explicit blob images; use nosniff and attachment handling from ENGINEERING.md. Allow only
the minimum inline styling needed for dynamic image-divider geometry; never inline scripts.
Framed tool pages require a later C3-specific policy. Do not weaken the foundation policy to
make a placeholder report work.

For initial progress, use an authenticated fetch stream of bounded newline-delimited
invalidation events (not a durable event log). Events say which snapshot changed; the UI fetches
authoritative state. On reconnect or overflow, refetch the snapshot. Sequence within a host
session; never use events as proof a tool call executed. No WebSocket framework or polling of
idle tool processes is required. Provide a small heartbeat for connection detection.

Proposed private host routes, distinct from tool schemas:

| Route | Purpose and typed boundary |
|---|---|
| `POST /api/session` | Single-use bootstrap exchange; returns browser token/host session ID |
| `GET /api/state` | Library summary, registered tool state, active attempts |
| `GET /api/events` | Authenticated invalidation stream; disconnect never cancels execution |
| `GET /api/tools/{id}/description` | Validated contract document plus host availability status |
| `POST /api/assets` | Stream one uploaded file; return opaque asset ID, digest, size, detected media |
| `GET /api/assets/{id}` | Authorized asset fetch; strict inline/attachment policy |
| `POST /api/attempts` | Idempotency ID, tool/action, description revision, parameters, asset-slot bindings |
| `GET /api/attempts?cursor=...&limit=...` | Stable newest-first pagination; default 50, max 100 |
| `GET /api/attempts/{id}` | Saved invocation, execution/preservation states, evidence references |
| `POST /api/attempts/{id}/cancel` | Idempotent cancellation request; unknown association stays explicit |
| `POST /api/examples` | Name/notes and source attempt ID; no draft arguments accepted |
| `GET /api/examples` | Paginated reusable snapshots; opening fills a draft without execution |
| `POST /api/cleanup/preview`, `POST /api/cleanup` | Selected IDs, computed references, preview revision checked on deletion |
| `POST /api/host/quit` | Active-attempt revision + explicit confirmation when active |

Rust serde structs with denied unknown request fields are authoritative for host DTOs. Keep
TypeScript types and shared JSON boundary fixtures together; generation is optional, not a new
framework requirement. Tool parameter objects remain governed by their supplied JSON Schema.
Same idempotency ID + different body returns 409; same body returns the existing attempt and
never dispatches again. Errors use `{error:{code,message,details?}}`: 400 malformed, 401 unauthenticated,
403 forbidden, 404 absent, 409 busy/stale/conflict, 413 oversized, 422 invalid data, 500 internal.

Foundation limits: upload 128 MiB/file, decoded image 128 megapixels and 512 MiB decoder memory,
JSON request 4 MiB, event line 16 KiB, and captured journal 16 MiB. Stream uploads and hash copies;
one image decoder at a time. Report limits clearly and mark excess journal preservation incomplete,
never silently truncate a supposedly complete record. Tool limits may be narrower and take precedence.
Supported initial image formats are PNG/JPEG; do not advertise arbitrary image codec support.

## Forms and component discipline

Support ordinary JSON Schema object fields, primitives, enums, required/optional values and
arrays. Complex unions/conditional structures can use an explicit JSON editor validated against
the complete tool schema; do not silently discard unsupported fields. File/image slots use ordered
asset arrays and explicit cardinality (including a max of one for v0.1 scalar slots), not a text input that gives the host arbitrary filesystem access. No CLI fallback.

Use one picker for task/operation/action variants, one drawer, and one image comparator. Keep
draft state separate from immutable saved attempts. A component enters the library only when a
real screen uses it; add stories for long labels, empty/error/loading states, keyboard focus,
320px narrow viewport, 200% zoom and reduced motion. No copy/paste forks per tool.

```rust
// Host request shape, not an addition to table-manners.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateExample {
    source_attempt_id: String,
    name: String,
    notes: Option<String>,
}
```

Use Rust module-specific error enums, no panics/unwraps on external input, parameterized SQL,
rustfmt and clippy. TypeScript strict mode, functional components, explicit state variants,
no `any` at API boundaries. Keep boundary tests meaningful rather than snapshotting implementation
details. Test names describe behavior, especially immutable run versus editable draft separation.

## Commands the plan will implement

M0's UI, Storybook, test and embedding commands are implemented. Host execution and dev-fake
commands remain planned for M1–M3. Run from the repository root. A JS toolchain is only for
development/UI regeneration.

| Command | Responsibility |
|---|---|
| `bun install --cwd ui --frozen-lockfile` | Reproduce the committed UI dependency graph after initial resolution |
| `bun run --cwd ui dev` | Vite UI development using an explicitly configured local host |
| `bun run --cwd ui typecheck` | `tsc --noEmit` with strict project settings |
| `bun run --cwd ui test:unit` | Bun tests scoped to pure UI/domain logic |
| `bun run --cwd ui storybook` | Component development on loopback port 6006 |
| `bun run --cwd ui build:storybook` | Build isolated component documentation |
| `bun run --cwd ui test:browser` | Playwright Chromium/WebKit projects against test servers |
| `bun run scripts/build-ui.ts` | Build UI, publish prebuilt files and digest manifest |
| `bun run scripts/check-ui.ts` | Rebuild to a temp directory; compare source/lock and output digests |
| `cargo fmt --all -- --check` | Rust format check |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Rust lint check |
| `cargo test --locked --all-targets` | Unit/integration tests with isolated temporary libraries |
| `cargo run --locked --bin wafflery -- --foreground --no-open` | Development host with embedded UI |
| `cargo build --locked --release --bin wafflery` | Installed-style binary; never invokes Node/Bun |
| `bun run scripts/dev-fake.ts` | Build/start independent fake + isolated host + browser-ready development URL |

UI build writes `ui/prebuilt/`; Rust embeds it using a build-time file enumeration that never
runs a JS command. Commit prebuilt files and their source/lock digest manifest intentionally
to satisfy source-only Rust builds. `build.rs` verifies the manifest's input hashes; missing prebuilt files always fail. Stale inputs
warn in debug builds (identify the embedded snapshot) and fail in release or when
`WAFFLERY_VERIFY_UI=1` is set. It never regenerates. UI development uses Vite while cargo tests
can continue; freshness is mandatory at checkpoint/release CI. F04a must prove byte-identical
output from two clean builds before digest equality is a gate. CI rebuilds and compares using
pinned versions, no timestamps/absolute paths in the manifest. Storybook is
excluded from embedded assets. Source snapshots include Cargo.lock, prebuilt files, and licenses.

## Verification and platform scope

The available development machine is Apple Silicon, macOS 27.0.1. Foundation validation targets
that machine, Chromium and WebKit automation, and a manual Safari smoke test. This is not a
claim of older macOS or Intel support. Set the public minimum OS/architecture matrix at the
distribution gate using actual builds/tests; no Linux work is on the foundation path.

Rust tests cover independent MCP wire exchange, protocol revision, file ownership, SQLite
transactions, cancellation uncertainty, duplicate dispatch and lifecycle. Playwright covers
component keyboard behavior, malformed assets, drafts/history, multi-tab state, reload, and
the first whole workflow. WebKit automation does not replace checking installed Safari.
Use owned fixtures and offline fake responses, never user libraries/keys in tests.

Chromium and WebKit caches are present, but must match the selected Playwright version and
actually launch. The current sandbox has blocked some network/process/browser actions in earlier checks.
Record blocked checks as not run and supply exact commands for a normal developer environment;
do not replace real-browser evidence with DOM mocks or claim a release checkpoint passed.
M0 must establish a usable install/test environment before implementation can claim verification.
