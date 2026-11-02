# Concurrent child pipe inheritance

The draft-edit full matrix exposed intermittent `Tool is disconnected` failures in unrelated
attempt tests. Twenty repeated suites reproduced it before any UI changes could be involved.
Temporary startup diagnostics located the earlier error: the configuration probe had not exited,
so the host correctly refused to start its serving session. The test setup then used an empty
revision and obscured that initial failure with the later disconnected message.

The MCP service reported successful cancellation and transport closure. Immediate `try_wait`
still found the child alive. A process sample showed it blocked reading stdin. File-descriptor
inspection proved that a different sample-lab process held both ends of that probe's stdin pipe;
the host held neither. Consequently host-side transport closure could not produce EOF. This
matches the non-atomic pipe/close-on-exec inheritance window documented in
[Rust issue #95584](https://github.com/rust-lang/rust/issues/95584). No assumption about lost
child-exit notifications or extra shutdown time is needed to explain the captured failure.

All production std and Tokio child creation now goes through a shared macOS mutex: tool probes,
serving tools, report groups, process observers, the detached host and browser opening. Unit-test
helper launches in the same process use it too. Only synchronous spawning holds the guard;
waiting, reading, tool execution and shutdown remain concurrent. This is scoped protection for
launches owned by Wafflery, not an OS sandbox or a claim that arbitrary third-party code uses it.
No dependency, timeout, retry, signal or force-stop policy changes.

The independent peer's `pipe-inheritance` scenario records open non-stdio pipe descriptors.
A regression starts 24 peers concurrently in eight batches, then closes each while its siblings
remain alive. Before the fix it found numerous inherited descriptors and eight EOF failures;
cleanup explicitly stopped only the test's still-owned children. The same regression and the
25-test attempt suite are repeated under the shared heavy-check lock after the fix.

This correction is one atomic ownership change spanning the spawn sites, regression and docs,
beyond the usual five-file target. Splitting its production coverage would leave a participating
spawn path outside the lock. Commit separately from the browser fixture correction; rollback the
spawn correction as a whole. No storage changes or sibling edits. Temporary diagnostics, process
samples and full descriptor listings remain outside the repository and are removed from code.

Verification: twenty rounds passed (3,840 peer launches and 500 attempt tests), with no inherited
pipes or EOF failures. Independent review approved. The full shared-lock matrix passed: 241 Rust
tests (three ignored), 41 UI unit tests, two asset tests, 180 Chromium/WebKit cases (four skips),
fmt, clippy, typecheck, Storybook, reproducible assets and release build. Review additionally
routed the independent wire-test peer launches through the guard; their final targeted checks
and fmt/clippy run are recorded in the execution checkpoints.

## Hosted oracle correction

Hosted run 41417569096 failed only the new regression: every peer reported the same descriptors
143 and 146, with no EOF failures. The old oracle classified every non-stdio pipe as a sibling
leak, although a caller may have passed pipes to the test process before any launch. Adding a
synthetic caller-owned pipe reproduced that false positive locally with the spawn guard intact.

The regression now snapshots pipe descriptors before the concurrent launches and reports only
new descriptors. The test retains its synthetic ambient pipe for its entire lifetime and requires
observations of both ends,
so this distinction is tested even on a machine without hosted-runner pipes. EOF ownership is
still checked with all siblings alive. Temporarily removing the Tokio spawn guard made the
corrected oracle fail on new descriptors and an EOF failure; the guard was restored afterward.

This is a test-boundary correction, not a product policy to close caller-owned descriptors.
The host still protects its own concurrent launches; it does not claim general descriptor
sandboxing. No production code or timeout changes. Rollback this test-only correction separately
from the spawn guard. Twenty stress rounds passed with ambient pipes present (3,840 peer
launches). Independent review approved. The full shared-lock matrix passed: 241 Rust tests
(three ignored), 41 UI unit tests, two asset tests, 180 Chromium/WebKit cases (four skips), fmt,
clippy, typecheck, Storybook, reproducible assets and the release build.
