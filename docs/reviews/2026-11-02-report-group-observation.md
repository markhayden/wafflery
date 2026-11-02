# Report group exit observation

Hosted run 41404918747 at 327b8e1 failed while observing a short-lived report leader.
The original test reproduced locally on iteration 15: macOS `ps` reported the leader
in state `?E`. The parser rejected that valid transitional state, rather than finding
an empty group or a missing leader.

Darwin's `ps` sources define `H` and `?` Mach states and append `E` for an exiting
non-zombie process. These states are now accepted as observations but never count as
drained. Only a complete observation containing the pinned leader and exclusively
zombie members permits reaping. Empty, missing-member and malformed observations
remain errors; they do not authorize releasing or signaling another process identity.

A related ordering defect could erase a previously received, one-shot NOTE_EXIT when
a later membership snapshot was uncertain. Exit knowledge is now monotonic. The
leader remains owned while uncertain descendants are observed.

References:

- https://github.com/apple-oss-distributions/adv_cmds/blob/main/ps/print.c
- https://github.com/apple-oss-distributions/adv_cmds/blob/main/ps/tasks.c

Two regression tests cover transitional leader/descendant states and preservation of
a confirmed exit event across an uncertain snapshot. All six group tests and 200
consecutive repetitions of the original early-exit test passed under the shared
heavy-check lock. No retries, timeout increases or relaxed exit confirmation were added.
Review approved the fix without blockers.

The full matrix passed: fmt, clippy, Rust tests, typecheck, 41 UI unit tests, two asset
tests, Storybook, reproducible UI asset checks, release build, and 164 browser cases
with four opt-in skips. Lockfiles and prebuilt assets are unchanged.

An earlier local matrix also encountered an estimate-shutdown timeout unrelated to
the failing group observation. That exact test passed 100 isolated repetitions and
the final full matrix. Its cause remains unconfirmed; this change does not claim to
fix it. Hosted confirmation remains pending the coordinator's push.

Rollback: revert this fix before dependent report changes. Document-output work stays
on its separate branch until the hosted repair is confirmed.
