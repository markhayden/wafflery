# Setup refresh browser ordering

Hosted run 41368632787 (83a5635) failed the WebKit test that keeps an unavailable tool's setup
separate from saved-run inspection. Rust checks and the other browser cases passed.

The downloaded trace and allowlisted attempt summary establish this sequence:

- The first run completed with complete contract evidence.
- The second tab's fill did not establish its required label; the field remained empty before
  clicking Run. No second POST to `/api/attempts` occurred, and only one attempt was recorded.
- The test expected that second run to invalidate the first tab's state. With no submission,
  its unavailable-state assertion timed out instead.
- The test only removed its state interception on success. During teardown, a subsequent
  intercepted state read reached the already stopped host and reported ECONNREFUSED.

There was no `/restart` request. This evidence does not support a host outage during tool
replacement. The exact WebKit text-insertion mechanism is still unconfirmed; the demonstrated
ordering error is treating a Run click as proof of a submitted run and leaving interception
active after the assertion failed.

The test now establishes and checks its initial draft, then uses the existing setup Refresh
control to trigger the state invalidation it actually needs. It waits for a response containing
the unavailable declaration before checking the drawer. Mutation is armed only by the explicit
Refresh request; an outstanding initial read cannot remove that control before the click.
Interception is drained in `finally`
before host teardown on success or failure. No production behavior, retries or timeouts changed.

Review approved the final ordering and cleanup. Under the shared lock, the final test passed
40 repetitions across Chromium and WebKit. The complete matrix then passed: formatting, clippy,
105 Rust tests (three ignored), typecheck, 18 UI unit tests, two asset tests, Storybook,
byte-identical UI builds, release build, and 86 browser cases (four real-tool skips). Lockfiles
and embedded UI assets remained unchanged. Hosted verification still requires a coordinator push.
