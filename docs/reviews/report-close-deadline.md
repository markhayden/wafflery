# Report close completion synchronization

Hosted run 41507896035 failed in the WebKit force-required report lifecycle test. The host
correctly retained the stuck renderer. The test's independent five-second phase poll raced the
close sequence: EOF gets two seconds, then SIGTERM gets another two seconds.

The uploaded trace shows the last poll observing `stopping` about 3984 ms after close dispatch.
The underlying close request returned `needs_force` at about 4009 ms. The next observation fell
just beyond the polling deadline. This is a test synchronization error, not evidence that the
host lost ownership or failed to stop gracefully.

The test now awaits the close HTTP response, which is produced only after `RenderProcess::close`
settles to `Exited` or `NeedsForce`. It verifies that the intercepted browser response still
contains the deliberately stale `stopping` observation, then checks that the host is actually
`needs_force`. Reopening the drawer must still recover the current state and require an explicit
force-stop confirmation. No production timeout, default assertion timeout or retry count changes.

Validation: the hosted trace preserves the failing case. All twenty WebKit repetitions passed
under the shared heavy-check lock. Independent review approved the completion-signal reasoning
and confirmed the remount/force-confirmation coverage remains intact. The full matrix passed:
fmt/clippy, 295 Rust tests (4 opt-in ignored), typecheck, 53 UI unit tests, 2 asset-tool tests,
Storybook, reproducible assets, release build, and 220 Chromium/WebKit tests (8 opt-in skipped).
An unrelated local preview occupied the usual test port; an untracked 4187 override was removed
after the checks. No lockfile, prebuilt asset or production code changed.
