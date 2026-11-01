# Squint offline integration checkpoint

Local date 2026-10-31; Squint #2 closed 2026-11-01 UTC. Verified its fix at `a0a2dde` from
an isolated `git archive` build with `--offline --locked --features testkit`. Rascal is the
previous isolated `9c52138` build. Sibling working files/build products were not changed.

## Delivered

- Strict schema discovery now passes with Squint's upstream fix. All scripted action cost
  declarations are free with no required credentials; the host's fresh dispatch guard is unchanged.
- The real scripted source-fidelity coordinator probe completes, records zero spend, saves
  complete evidence and leaves input originals unchanged. Its verdict is fixture data only.
- One browser workspace runs Squint and Rascal, preserves their independent results, exports
  evidence, opens a saved example without executing, and restores history after restart.
- Data-only results show the saved result body without an empty image comparison. A missing
  declared output still uses the comparison's missing-image behavior.
- `scripts/dev-pipeline.ts` accepts explicit executable paths, creates an isolated offline
  library, and exercises the actual `squint playground` H1 entry. Launch exited successfully;
  the new host answered its control status request. Existing personal/Rascal libraries are untouched.

## Verification

- Required Rust: 63 passed, 0 failed, 3 explicit-binary tests ignored by default.
- Optional `real_squint_scripted`: passed against `a0a2dde`.
- UI: typecheck and 10 unit tests passed. Storybook builds in the browser runner.
- Targeted data-only/missing-evidence/two-tool browser checks: 6 passed in Chromium/WebKit.
- Full browser run: 62 passed; the two new Squint cases failed on a test-only race where an
  extra Close click targeted the already auto-closing history drawer. Replaced it with an
  assertion that the drawer is hidden; both corrected cases passed on the targeted rerun.
  All 64 cases therefore have passing verification, without claiming a second full-suite run.
- Browser screenshot inspected after history restoration; data fits the result pane.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`, formatting, diff check,
  release build and UI manifest verification passed. Two clean UI builds are byte-identical.
- Independent `/root/m2_review` approved, no blocking findings; reviewer did not rerun tests.
  The later test-only history-dismissal correction and verification results were sent to reviewer.

## Remaining boundaries

The optional estimate probe executes successfully but fails its complete-evidence assertion:
Squint's estimate has no readable journal. It remains ignored and strict, separately from the
passing scripted probe. See [the prepared follow-up](../handoffs/squint-estimate-journal.md),
which has not been posted publicly. This does not block scripted checks.

No live model judgment or paid call was made. Reference-list bindings, config/key readiness,
C2 reproducible exports, C3 tool reports, operations and the final navigation hierarchy retain
their existing gates. A JSON result-body preview is a generic fallback, not the final tool report.
No new native Safari claim; the owner's previous M3 Safari check remains the native evidence.
