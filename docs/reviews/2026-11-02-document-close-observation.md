# Document close observation

V17b2's full matrix exposed a pre-existing assertion in the idle-expiry test: after the
monitor confirmed closure, a new TCP connect returned success. Repetition also failed the
last-owner-drop assertion. The diagnostic recurrence occurred in the content-policy close test:
`lsof` showed only CLOSED/CLOSE_WAIT client sockets, no listener, and writing the old document
request failed with `BrokenPipe`. This evidence does not establish a leaked listener or early
closure publication.

The implementation drops the listener, aborts and joins accepted connections, then publishes
content closure. The document monitor awaits that confirmation before publishing its closure.
A successful TCP connect alone is therefore the wrong failure oracle: a socket whose handshake
finishes can already be closed. The tests now accept connection refusal or an immediately
closed connection (EOF/reset). A connected socket that remains open or yields any data still
fails, with a bounded read. There are no retries, longer existing timeouts, or product changes.
All six document/content assertions share this rule. A real connected pair verifies EOF after
peer closure; a temporary leaked-listener mutation is used to prove the rule still detects a
server that remains open.

Verification: thirty complete library-suite rounds passed (2,970 tests). Temporarily retaining
the listener with `mem::forget` made the idle-expiry check fail with a still-open socket; the
mutation was removed. Independent review approved. The full shared-lock matrix passed: 243 Rust tests (three ignored),
51 UI unit tests, two asset tests and 180 browser cases (four opt-in skips), plus fmt, clippy,
typecheck, Storybook, reproducible assets and the release build.
