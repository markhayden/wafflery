# Hosted browser teardown lifetime

Run 41376551726 at 5bf59d3 failed in WebKit's cancellation scenario. The downloaded trace
shows every test-body assertion passed, including the final unconfirmed-outcome drawer check.
The error begins in the host fixture's After Hooks, approximately 269 ms after teardown starts.
Startup's control-status handshake had succeeded. This is not evidence of a missing startup
handshake or cross-worker temporary-directory collision: each fixture uses a unique library root,
and the socket name hashes that canonical root.

The fixture sent SIGTERM, waited 250 ms, then requested a new bootstrap credential through the
control socket. Rust removes that socket while dropping host ownership before process exit;
Node need not have delivered the child exit event yet. Checking `child.exitCode` in the catch
handler therefore did not make a missing socket safe. The same lifecycle also mishandled signal
exits, for which exitCode remains null. Failure diagnostics ran before teardown and consequently
did not capture this post-test failure; the retained Playwright trace supplies the ordering.

A shared HostLifetime now tracks closure from spawn time for both browser host fixtures.
Teardown credentials are acquired after readiness and before tests can shut the host down.
Stopping never needs private IPC again. Shutdown HTTP failures wait for confirmed clean process
closure; they do not imply success by themselves. Force quit still requires an observed waiting
state. Concurrent stop calls share one promise, abnormal exits fail explicitly, and a 10-second
teardown bound stays below the existing 15-second test limit. This replaces an unbounded final
exit wait; it does not extend browser assertion timeouts or add retries. Stderr is drained so a
full pipe cannot block process termination. Temporary directories are removed only after closure.

Deterministic unit cases model transport disappearance before exit and close, UI-initiated
closure, signal death with null exitCode, and concurrent stop calls forcing only once. These
failed against the unimplemented lifetime boundary before the helper was implemented. The
hosted trace, rather than that stub failure, establishes the old fixture's specific race.

Sources: Node's child-process documentation distinguishes exit from close (stdio closed) and
states that signal termination leaves exitCode null. Playwright's fixture execution-order
documentation specifies teardown after use returns. No product or shared-contract change.

- https://nodejs.org/api/child_process.html#event-close
- https://nodejs.org/api/child_process.html#subprocessexitcode
- https://playwright.dev/docs/test-fixtures#execution-order

The shared heavy-check lock isolated validation from sibling builds. All 200 browser repetitions
passed (cancellation, shutdown and session specs, 20 repeats in both engines, zero retries).
Four deterministic lifetime tests and typechecking pass. Review approved without blockers.

Full matrix passed: 129 Rust tests (three ignored), 27 UI unit tests, two asset tests and 100
browser cases (four opt-in skips), plus fmt, clippy, typecheck, Storybook, asset reproducibility
and release build. No generated assets or lockfiles changed. Hosted confirmation requires the
coordinator's push. This fix lands on top of the locally verified V09a change (57465dc).
