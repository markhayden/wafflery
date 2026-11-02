# Registration ownership release

Hosted run 41374071471 failed in `tests/registration.rs`: acquiring ownership immediately after
successful registration returned WouldBlock. No browser tests ran and no diagnostics artifact
was uploaded. The log does not identify which process still held the descriptor.

A deterministic regression retains a duplicate of the owner's descriptor, drops ownership, and
tries to reacquire it. Before the fix this returns the same WouldBlock.
[Rust's lock documentation](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)
specifies that closing a descriptor releases the lock only after all duplicated/inherited
descriptors close; explicit unlock releases it directly. Concurrent process creation in the
registration test binary is a plausible source of transient inheritance, not a captured fact.

Use a private ownership-lock guard that explicitly unlocks on drop. Install it immediately after
locking, so both successful ownership destruction and later acquisition-error paths release
the lock. Discovery/socket cleanup runs before the guard drops. The regression also proves
that closing the old duplicate cannot release a replacement owner's independently acquired lock.
Preserve underlying lock-system errors instead of labeling every failure WouldBlock, so future
failures retain diagnostic evidence. No retries, timeouts, dependencies or launcher ABI changes. Roll back this guard and its
regression together if needed. Full matrix and repeated registration checks gate the merge.

The first local full matrix also saw `Tool is disconnected` before dispatch in the unrelated
cancellation-pagination test. Twenty isolated repetitions passed unchanged; no root cause was
established for that observation. It is not claimed fixed by the lock change. The complete
matrix is rerun before merge, with this separate observation retained for follow-up if it recurs.

Final verification passed: the deterministic regression, 20 complete registration-suite runs,
121 Rust tests (three ignored), 23 UI unit tests, two asset tests and 92 browser cases (four
skipped), plus fmt, clippy, typecheck, Storybook, asset reproducibility and release build. Review
found no blockers. Hosted CI confirmation remains with the coordinator.
