# macOS continuous integration

Approved scope: the coordinator requested the existing PLAN check matrix on GitHub Actions,
2026-11-01. No application behavior, shared contract or deployment changes.

## Specification

On pull requests, pushes to main and manual dispatch, run the full required matrix on `macos-15`.
Use full commit pins for actions, the repository's Rust toolchain, Bun 1.4.3 and Node 22.13.1.
Cache Cargo downloads/builds and Bun's package cache; install dependencies from frozen lockfiles
and install matching Chromium/WebKit browsers. No sibling checkouts, provider keys or paid calls.
Opt-in real-tool probes stay opt-in; CI's mandatory execution coverage uses the independent peer.

Keep repository permissions read-only, checkout credentials unpersisted, concurrent obsolete
runs cancellable and job duration bounded. Never upload browser traces automatically: they can
contain disposable browser credentials. Logs remain in the job; no publication or deployment.
Verify checked-in UI assets before any browser command can regenerate them.

## Ordered tasks and commits

1. **CI01 — Record scope and rollback.** This document establishes acceptance and the existing
   verification commands. Commit `docs: specify the macOS CI gate`.
2. **CI02 — Add the workflow.** One workflow, matching README instructions and a verification
   checkpoint. Validate YAML/expressions/shell with actionlint, inspect action pins against their
   upstream repositories, then execute the full PLAN matrix locally with `CI=true`. The workflow
   reuses existing behavioral tests; it adds no duplicated implementation tests. Commit
   `ci: run the full check matrix on macOS 15` after checks and review.
3. **Checkpoint.** Merge locally; the coordinator reviews/pushes and observes the first hosted
   run. Local checks establish command correctness, not a successful hosted run or branch
   protection. No push or tag from this work.

Rollback CI02 with `git revert` of its commit; application/runtime/data behavior is unaffected.
CI01 can remain as the decision record. A failed hosted run is corrected in another reviewed
commit, never by disabling an existing gate. Runner or registry availability is reported explicitly.

## Sources

- [GitHub runner image](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-Readme.md)
- [Action caching](https://github.com/actions/cache)
- [Bun setup](https://github.com/oven-sh/setup-bun)
- [Playwright CI](https://playwright.dev/docs/ci)

All action commit pins are resolved from upstream tags during implementation. The local machine
is a newer macOS version; the first hosted macOS 15 run remains separate evidence.
