# V-B: estimates and bounded paid execution

V08–V13 build on the setup checkpoint with the published table-manners =0.3.0 crate.
The independent peer's paid scenarios were introduced at 253b1ba; the selected-outcome
presentation is 697d4eb. The checkpoint test extends that host without changing production code.
The contract source and registry checksum remain those recorded in the V-A checkpoint.

A paid action requires a positive explicit cap, an allowed estimate and the exact owned
preparation. Estimates remain controls: they create neither attempts nor history. Draft,
readiness, session and expiry changes invalidate quotes. A ledger-only balance change does not;
the tool can refuse the later call for budget. Transport permits enforce the opaque binding
alongside the recorded arguments. Submission uncertainty retains its original identity for an
explicit retry; no automatic execution or session restart occurs.

The independent peer covers stale estimates, cap and budget refusal, missing setup before a
run, provider authentication rejection during a run, known charges and unresolved worst-case
spend. Held paid calls reserve funds while control requests remain responsive. Cancellation
before allocation retains the claim; cancellation after a synthetic provider request preserves
unresolved spend. Readiness refresh after authentication/setup outcomes does not replay work.
One active execution per tool remains enforced.

The browser checkpoint follows cap → estimate → run → cancel → history → evidence archive.
After editing the draft, it reopens the cancelled run and checks the exported original inputs,
cap, opaque binding, result and settled session ledger. It then stops and starts the host,
reopens the same history, and verifies that the original session and unresolved spend survive
while the new serving session has a fresh ledger. Only one synthetic paid call is recorded.
Existing integration and browser coverage separately verifies active-work refusal, responsive
controls, session-restart invalidation, sequential budget changes and readiness-driven invalidation.

The initial checkpoint passed in Chromium and WebKit. Review strengthened the binding and new
session assertions so missing fields or a zero reservation cannot masquerade as success. The
first full matrix then caught an existing lost-acknowledgement test whose offscreen label fill
had not changed the input before retry. Shared label editing now clicks, checks focus and
verifies the value before continuing; no retries or longer timeouts were added. All 120 acknowledgement repetitions passed
(20 per case in Chromium and WebKit). The final shared-lock full matrix passed:
183 Rust tests (three ignored), 41 UI unit tests, two asset tests, 164 browser cases (four opt-in
skips), fmt, clippy, typecheck, Storybook, reproducible assets and release build. Review approved the test changes. No provider or network
billing call is made by the mandatory suite. Real peer/provider acceptance remains V18.
Native Safari was not rerun; automated WebKit is reported separately from a manual Safari check.

Rollback this test/documentation slice first, then V13 presentation and observer changes before
V12's dispatch changes. No library migration is introduced; retain saved attempts and assets.
