# M4: real tool entry

Status: authorized continuation, 2026-10-31. Work starts with explicit local registration and
the real Rascal workflow; automatic discovery and provider setup retain their interface gates.
The owner also permits paid runs. That permission does not make the current client implement
estimates, caps or a session ledger; those remain required before enabling paid dispatch.

## Current evidence and decisions

- Rascal M9 is merged; reviewed launcher source passes exactly `--tool rascal`, uses
  `WAFFLEHOUSE_BIN` and leaves host attachment/browser ownership to Wafflehouse. Current local
  binary describes contract 0.1.0. No-config `seam` can emit a mask without credentials/filler.
- Table-manners at `b260527` contains a 0.2.0 draft but only a v0.1.0 tag. Keep the supported
  version set unchanged until the tagged contract and tools are adopted together.
- Squint has active uncommitted changes. Read-only source inspection is allowed; no sibling edits.
- Core startup has no agreed automatic discovery descriptor. The existing host descriptor
  (`id`, absolute executable, argv array, cwd and roots) can be installed explicitly, without
  host code interpreting Rascal/Squint private settings or inventing new tool flags.
- Registration is setup only, not a command-line alternative for running experiments. It never
  executes a tool or installs a binary. Require a stopped host and its lifetime lock; never
  rewrite live sessions. New records cannot silently overwrite an existing tool ID.
- Add host-owned `WAFFLEHOUSE_DATA_DIR` for consistent isolated entry from either launcher.
  Explicit `--data-dir` wins. Tools continue to pass exactly their existing two arguments.
- The Rascal development helper is explicitly tool-specific test/setup material; production
  host/form/result code stays generic. Use an owned fresh tool workspace, no config/filler and
  no inherited credentials. Do not adopt an unrelated existing tool workspace.

## G01 — Persist explicit tool registrations

- Accept: bounded regular-file JSON descriptor list, validate all incoming records before writing;
  canonical absolute executable/cwd/roots, executable permission, unique IDs and max 32 tools.
  An identical registration is idempotent; a changed record with an existing ID is refused.
- Accept: acquire the normal host lifetime lock, append atomically with a private synced file,
  preserve existing records and reject symlinks/malformed input without changing configuration.
- Verify: registration tests cover spaces, symlinks to executable, no tool execution, conflicting
  IDs, busy host lock, malformed/oversized input, output permissions and unchanged prior bytes.
- Files: registration module, new setup module, integration tests. No dependencies. Small slice.

## G02 — Expose registration and shared workspace selection

- Accept: `wafflehouse --register <descriptors.json> [--data-dir <path>]` registers then exits;
  incompatible launch flags are usage errors. No browser/server starts during registration.
- Accept: normal startup resolves explicit data-dir, then `WAFFLEHOUSE_DATA_DIR`, then the
  personal default. Blank environment overrides fail clearly; existing H1 argv is unchanged.
- Verify: subprocess CLI tests exercise registration, malformed combinations, data-dir precedence,
  and the tool-selector path. Existing concurrent launcher/lifecycle tests remain green.
- Files: launch options, main, help, CLI tests. Depends G01. Small slice.

## G03 — Run real Rascal in the browser

- Accept: an explicit-path helper builds only Wafflehouse, registers Rascal in a fresh owned
  workspace via G02, and enters through the actual `rascal playground` launcher. Print a safe
  reopen command with the same data directory. A no-open option uses direct host startup.
- Accept: optional browser coverage uses the real installed executable and generic UI to choose
  seam, upload an owned panorama, run, inspect saved evidence, save/open an example and download
  image evidence. Restart/reopen preserves the run. Source bytes are unchanged; reported spend zero.
- Verify: Chromium/WebKit real-Rascal browser run plus the existing optional coordinator smoke.
  Require a deliberately selected binary; no background install, no real providers. If no binary
  is selected, mark these optional checks skipped explicitly.
- Files: setup helper, browser fixture extension, Rascal browser test, README. Depends G02.

## G04 — Review the entry slice and expose remaining gates

- Accept: record actual binary/version/contract and invocation evidence, source audit revisions,
  implemented journey and unsupported capabilities. Open the real Rascal workspace for owner use.
- Verify: targeted checks then relevant complete suites, embedded assets reproducible, review diff;
  report native Safari separately. No automatic merge past the pending M3 Safari gate.
- Files: checkpoint, README/testkit docs, this plan/H1 documentation. Depends G03.

## Follow-on interfaces, not implied complete by this slice

- Automatic installed-tool discovery/startup declaration: agree an explicit machine-readable
  format with tool owners before adding detection/install UI. Manual registration is the bridge.
- C1: distinguish control actions such as estimate from run actions; expose readiness/provider
  configuration, typed choices and input-bound estimates without private-field heuristics.
  The current generic catalogue may expose actions whose dependencies are not configured.
- Location input and Operations: required location actions remain unavailable in this slice;
  R3 is now scheduled in Rascal M10. Wafflehouse must review its profile before implementing it.
- Paid runs: adopt tagged C4 with the tools, display estimates and session allowance, validate the
  chosen cap at dispatch, preserve actual/uncertain spend through failure and cancellation. Start
  live verification with a deliberately small explicit cap. No provider balance inferred from keys.
- Installer release, C2 native replay and C3 report pages remain separately gated. Squint follows
  Rascal using the same generic host once its supported configuration is ready.

M4 entry work can proceed while M3's native Safari checklist remains outstanding. A general
visual approval is not a claim that its full manual workflow check passed.
