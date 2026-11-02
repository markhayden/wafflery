# v0.3 report checkpoint (V-C)

V14–V17 resolve report commands from saved runs using the registered tool executable, own
renderer process groups through graceful close and confirmed force, validate readiness, and
serve copied standalone documents on an isolated origin. One-use frame grants feed a trusted
wrapper whose exact-origin policy contains the untrusted report. Existing browser checks prove
blocked parent access, unauthenticated API calls, form submission, popups and foreign navigation;
this is browser isolation, not an OS sandbox for the executable.

The Reports tab preserves ownership across drawer lifetimes. Opening is explicit; a picker
selection closes an old view without starting its replacement. Leaving Reports releases the
view. Remount resumes observing a retained closing/force-required view without reopening or
renewing it. Lost open acknowledgement stays uncertain; exact-identity recovery is available
for a lost close. Saved inputs, results and spend never change because a report fails.

V17b3 browser cases use real independent-peer renderer processes for generation dismissal,
server-to-document replacement, reopening and confirmed force after a stuck renderer. The
peer emits its readiness diagnostic only after its signal handler exists, avoiding a close
racing initial signal setup. The force test deliberately delivers an old closing observation
so reopening must read the current needs-force state. Intercepted requests are drained before
fixture shutdown. Keyboard focus returns from the embedded document to host controls before
Escape dismisses the drawer; key events inside the foreign-origin report itself do not reach
the host. macOS WebKit uses Option-Tab for controls, as in the foundation checks.

The visibility case drives the browser's visibility boundary and timers deterministically:
visible inspection sends activity, hidden inspection sends none, and returning observes an
expired view without opening a replacement. Its expiry response is simulated after a real
close; it is not claimed as a five-minute wall-clock browser test. Rust's paused-clock tests
separately prove the actual fixed five-minute lease, renewal, non-resurrection and autonomous
process/content-server shutdown. No production TTL override was added.

Verification: focused cases and 160 lifecycle repetitions passed in Chromium/WebKit, with
independent review. The full shared-lock matrix passed: 243 Rust tests (three ignored), 53 UI
unit tests, two asset tests and 198 browser cases (four opt-in skips), fmt, clippy, typecheck,
Storybook, reproducible assets and release build.
Native Safari manual verification remains pending, separately from automated WebKit. The focused
walkthrough is: run the independent peer in its renders scenario; open server and document
reports from a saved run after editing the draft; return to evidence and reopen; dismiss during
generation; confirm a stuck viewer requires explicit force; verify saved evidence remains
available after a failed report. Real Rascal/Squint acceptance follows in V18 using the approved
9ee6bc8/db5cf78 isolated sources. No real paid provider call is part of this checkpoint.

Rollback this verification slice first, then drawer controls and ownership before render host
changes. Preserve saved runs and tool-owned output. No library migration is introduced.
