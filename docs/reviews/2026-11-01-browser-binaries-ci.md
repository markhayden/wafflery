# Build browser-test executables before workers

Hosted run [41365178137](https://github.com/markhayden/wafflery/actions/runs/41365178137)
on aaf5b5d passed Rust and 77 browser cases. The WebKit workspace test completed its first run,
saved complete evidence without notices, stopped the host, then failed to spawn the debug host
for its restart (`ENOENT`). The debug host had been built before browser execution, so this was
not an initial omitted build. Logs and downloaded trace/evidence were inspected.

Each tool-host fixture also invoked Cargo for the host and peer. Those builds ran during other
workers' tests and restarts against the same executable paths. Concurrent Cargo replacement is
the inferred source of the transient missing path; the artifacts do not contain a filesystem
unlink trace. Regardless of cache freshness, building shared executables during their use is
an unsafe test boundary and unnecessary work.

The browser command now explicitly builds both `wafflery` and `sample-lab` once before launching
Playwright. Fixtures never invoke Cargo. Playwright's global setup verifies both paths are
regular executable files, failing with the precise path and build command before workers start.
This also gives direct Playwright invocations an actionable missing-build error. No retries,
timeout increases, workflow cache resets or production changes are needed.

The three negative availability tests failed before validation was implemented. They cover a
missing host, a missing peer, a directory, and a non-executable file; a valid pair passes.
The new setup and ordering passed review. Workspace and session browser files passed 20
repetitions in both engines: 200 cases with two workers, without retries or changed timeouts.
The full matrix passed under the shared lock: formatting, clippy, 89 Rust tests (three opt-in
ignores), typecheck, 18 UI unit tests, two asset-tooling tests, Storybook, two byte-identical clean
UI builds, release build, and 78 browser cases (four real-tool opt-in skips). Lockfiles and
prebuilt outputs stayed stable after regenerating the input manifest for the script change.
Hosted verification awaits the coordinator's push. No providers, sibling edits, push or tags.

Rollback this focused tooling change as one commit. V04b remains on its feature branch.
