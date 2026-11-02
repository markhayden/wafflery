# Readiness completion and hosted CI

Hosted run [41363640476](https://github.com/markhayden/wafflery/actions/runs/41363640476)
failed at the Rust readiness test on main 526254a. The refusal was already saved, but the test
submitted its corrected call before materialized-input cleanup released the tool reservation.
The UI already observes `tool.busy`; terminal database evidence alone is not that signal.
The run stopped before browsers and contains no uploaded browser artifacts.

Correction: wait for terminal evidence and `tool.busy == false` under the existing test deadline,
then submit once. Do not retry the submission or increase its timeout.

Review also found a product handoff race: successful execution released its reservation before
unconditionally clearing the dispatch gate. A next call could install authorization during that
window, then lose it to the old worker. Defer successful release to final cleanup and revoke only
authorization whose attempt ID matches the finishing worker. Keep uncertain outcomes reserved.
A deterministic transport regression installs the next authorization before the previous cleanup
runs and verifies the next request is persisted and sent. Own abandoned authorization is revoked.
Completion invalidation remains after reservation release.

The first full local matrix also stopped at an existing cancellation test's immediate `close()`
assertion. During diagnostic repetition, a separate evidence test demonstrably called `close()`
while the coordinator still protected preservation. Ordinary execution-test teardown now uses
`begin_shutdown` and waits for the existing quit signal under the established eight-second
shutdown-test bound. It never force-stops or retries dispatch. Dedicated shutdown behavior tests
are unchanged. The cancellation-only case passed 20 diagnostic repetitions with completed
transport closures and child exits; its earlier single close failure was not reproduced, so
that particular cause is not claimed as established. Temporary diagnostics were removed.

Verification passed under the shared heavy-check lock. The authorization regression failed
before the matching-ID correction, then passed. Twenty rounds of all 17 execution tests, both
readiness tests and the authorization regression passed (400 checks). The full matrix passed:
formatting, clippy, 89 Rust tests (three opt-in ignores), typecheck, 14 UI unit tests, two asset
tooling tests, Storybook, two byte-identical clean UI builds, release build and 78 Chromium/WebKit
cases (four real-tool opt-in skips). Assets and lockfiles remained unchanged. Independent review
approved the product correction and teardown changes. Hosted verification awaits the coordinator's
push. Roll back this focused correction as one commit. V04b remains on its own
branch; no contract changes, provider calls or sibling edits are needed.
