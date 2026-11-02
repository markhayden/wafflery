# V-A: setup and image inputs

V01–V07 are complete at host f263f1b. The independent peer last changed in 7e5fe79.
The host uses the published table-manners =0.3.0 crate, source
9ac2142bf9315f4c5a0eb5935aa94da9bc95a697, registry checksum
9bec37d99d246d9b9d2a05da9c9be30ca3951773e3f762c3b8f5c14fa2360c79.
All eleven schemas match the reviewed e503a23 contract.

Offline actions, readiness/setup display, explicit tool-session replacement and scalar/list
image bindings work against the independent peer. Saved examples and history preserve each
image occurrence's order and coordinates. Paid dispatch remains refused. This checkpoint
covers M4's readiness/session portion; automatic discovery, production installation and the
three first-install journeys remain outside it. Real v0.3 tool acceptance belongs to V18.

The full matrix passed at f263f1b: fmt, clippy, 117 Rust tests (three ignored), typecheck,
23 UI unit tests, two asset tests, Storybook, reproducible UI assets, release build and
92 Chromium/WebKit cases (four skipped). The delayed-upload ownership regression also
passed 40 repetitions. Review approved the implementation after its import-race correction.
Responsive screenshots were inspected at desktop and mobile widths. Native Safari was not
rerun for this checkpoint; automated WebKit does not stand in for that manual check.

Rollback: revert dependent V07 UI changes before binding changes. No library schema migration
was introduced. Preserve local assets and library data. The stable checkpoint before paid
execution is this host with the paid-action guard still enforced.
