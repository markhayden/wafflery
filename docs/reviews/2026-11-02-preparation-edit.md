# Held preparation edit verification

V16a's first matrix passed Rust and build checks, then failed WebKit's existing
held-preparation edit scenario. The trace shows the label remained `prepare held`
immediately after `fill("new draft")` returned. The test released the held response
without checking either the edited value or the app's invalidation state. The app
then estimated the unchanged draft. This is not evidence of accepting a stale receipt
after a committed edit.

The three label-edit scenarios in the estimate suite now use the existing
`editSampleLabel` helper: click, confirm focus, fill, confirm value. Held preparation
and held estimate responses additionally wait for the visible stale-pending message
before release. This proves the intended edit happened before testing the late reply.
The shared helper already addresses the same offscreen WebKit input behavior in other
suites; no product code, assertion timeout or retry policy changes.

Verification is recorded with the V16a execution checkpoint. The failure trace is
kept outside the repository because it contains temporary browser session credentials.
