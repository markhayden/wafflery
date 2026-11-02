# V18: real v0.3 interoperability

This records the earlier V18 build. M5.02 replaces roots with explicit argv grants, so the Squint
`54b1f41` commands here are unsupported on that intermediate build; M5.03 adopts the v0.4 peers.

V18a–c automated interoperability checks are complete. Native Safari and billed-provider limits remain below.

The automated offline checkpoint uses owner-approved Rascal `9ee6bc8` and Squint `54b1f41`.
The latter supersedes `db5cf78` after the coordinator-approved viewer CSSOM/class fix.
Both were built from isolated source archives with locked dependencies and `testkit` enabled.
Sibling working trees were neither edited nor built in place. The host uses published
`table-manners =0.3.0`, MCP 2025-11-25 and the same generic client used by the independent peer.

## Verified paths

| Path | Evidence |
|---|---|
| Rascal without configuration | Ready/free seam; fingerprinted source and output mask; zero spend; complete saved contract evidence |
| Rascal selected configuration | Local scripted filler; configured cap error, lower cap refusal, accepted bound call and synthetic $0.04 spend, subsequent session-budget refusal |
| Rascal restart | Edited config remains inactive until confirmed restart; new session/price/config identity and zero ledger; old quote invalidated; saved run unchanged |
| Squint setup | Missing config remains describable; default `./squint.toml` and explicit selected path containing spaces; config change requires restart; missing credential is visible |
| Squint estimate | Real-profile catalog pricing without its credential; readiness refusal; no host attempt, tool run or spend; restart invalidates preparation |
| Squint references | Scripted reference-consistency verdict; three references in order with a repeated asset; distinct execution paths; matching saved and result fingerprints |
| Both report kinds | Rascal dash/page and Squint report/export; exact frame ancestor, separate origin, explicit open and graceful close, unchanged saved evidence |
| Chromium and WebKit | Real tool runs, image evidence, exports, examples, tool switching, draft edits, nested report frames, observed report exit and host restart |

The ordinary UI receives no nontransparent estimate binding. The accepted Rascal call carries the
host-owned binding and the saved observation matches that recorded wire. The $0.04 is a declared
fixture cost, not a charge. The same executable is used for serving and rendering. No tool-specific
production client behavior or broader environment/PATH inheritance was added.

Rust probes require explicitly selected binaries and isolated XDG config/data/cache directories.
Browser fixtures create their own XDG directories for each host and retain them across that host's
restart. Temporary render documents remain inside those owned cache directories. Rascal receives
only its library execution directory through `serve --allow-dir`; the approved Squint revision
predates its equivalent rule. Newer peer revisions require a separate approved acceptance pass.

## Reproduction

Follow [testkit setup](../../testkit/README.md) for isolated XDG directories and absolute binary
selection. Run heavy checks through the shared project lock when other tools are building.

```sh
env -u ANTHROPIC_API_KEY cargo test --locked --test rascal_smoke --test squint_smoke -- --ignored --test-threads=1
bun run --cwd ui test:browser rascal.spec.ts squint.spec.ts real-peer-reports.spec.ts
```

The two `WAFFLERY_*_BIN` selections must point to the approved isolated builds. No binary is
silently discovered or installed; a bad explicit Rust selection fails. Browser journeys are
skipped by the default matrix when binary variables are absent. The full required matrix still
uses the independent peer, so hosted CI does not depend on sibling builds or provider credentials.

## Limits

- Scripted verdicts do not measure model quality. No billed provider call or live credential
  validation was performed. Real provider 401/403 and actual billing remain unverified here.
- Playwright WebKit passed; native Safari report/frame/restart checks remain pending separately
  from the owner's earlier M3 Safari pass. See [V-C](v03-reports-checkpoint.md).
- Report screenshots were inspected locally and retained as test artifacts. They include generated
  run paths/identities and are not committed as public visual baselines. V19 uses stable synthetic
  data for public screenshots and regression baselines.
- C2 portable replay and R3 independent operations remain their separately scheduled integrations;
  this checkpoint does not claim those interfaces are delivered.

## Resolved server-viewer issue

The strict console pass found [Squint's server CSP/style conflict](../handoffs/squint-report-csp.md)
on `db5cf78`. The approved `54b1f41` fixes the viewer through CSSOM/classes without relaxing CSP.
Chromium and WebKit now pass unfiltered console/page-error checks and four distinct nontransparent
legend swatch assertions. Playwright's WebKit screenshot preparation itself inserts an inline
`body {}` stylesheet into every frame, so optional screenshot capture is excluded from this
strict test. The trace located the remaining violation exactly inside that screenshot call;
no console message is filtered or cleared.

## Verification record

Four Rust real-peer probes and all eight real-tool browser journeys passed again against the
approved fixed snapshots. Independent review approved fixture isolation, lifecycle assertions,
strict console checks and exact evidence preservation. Full-matrix results are recorded with
the V18c execution entry.
