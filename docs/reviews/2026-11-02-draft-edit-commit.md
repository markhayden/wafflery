# Draft fixture edit commitment

Hosted run 41412340374 failed the WebKit wrong-paid-acknowledgement scenario. Focus had
passed, `fill()` returned, and both the input value and React-rendered value attribute remained
`reviewed paid draft`. This trace does not identify what disturbed native insertion. One hundred
local repetitions of the unchanged scenario passed; the hosted trigger is not claimed reproduced.

The installed Playwright 1.64.1 implementation selects/focuses a text input in its injected
script, then sends a separate WebKit `Page.insertText` command. A deterministic regression adds
a select-event listener that blurs the input during that gap. The old helper reproduces the
same failure: focus passes and fill returns, but the original value remains. This establishes
a gap in the fixture helper, not evidence of the application discarding a committed input event.

The helper prepares synthetic test data in one DOM task using the native input value setter
and a bubbling input event. It then requires both the live value and React's rendered value
attribute to match. It does not write the value attribute or application state directly. The
regression runs the tool and verifies the saved input, proving the change reached the app.
Separate browser tests use native keyboard selection/typing and check the same saved evidence,
including the uncertain paid-acknowledgement state and its original-request retry;
protocol tests no longer depend on native text insertion to prepare their fixtures.

No production code, assertion timeout or retry policy changes. This supersedes the earlier
focus-only helper adjustment. The unchanged hosted job passed on rerun, confirming intermittency.
Verification: 380 WebKit fixture/protocol cases and 40 paid-state native-keyboard repetitions passed.
The full shared-lock matrix passed: 241 Rust tests (three ignored), 41 UI unit tests,
two asset tests, 180 browser cases (four skips), fmt, clippy, typecheck, Storybook,
reproducible assets and the release build. Independent review approved. The matrix also
exposed a separate [probe spawn race](2026-11-02-spawn-isolation.md), fixed in its own commit.
Hosted diagnostics remain outside the repository because traces contain temporary credentials.
