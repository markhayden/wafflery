# Canonical product rename checkpoint

Implemented on `rename-wafflery` from main `3836cd6`, 2026-11-01.

## Result

Cargo package/library/binary, UI identity and logo paths, browser storage and upload headers,
evidence archives, environment variables, scripts, tests, stories, mocks and documentation use
the canonical Wafflery name. H1 selects `wafflery` through `WAFFLERY_BIN`, PATH, then
`~/.local/bin/wafflery`; tool argv remains exactly `--tool <id>`. Sibling launcher changes belong
to the coordinator. Development helpers explicitly require those rebuilt launchers.

The new default parent is `~/Library/Application Support/Wafflery`; the owned SQLite/image
library is under `Library`. No aliases, old environment fallback, automatic migration or folder
rename were added. The previous default parent did not exist on this machine, so there is no
one-time library move to perform. Existing temporary libraries were left intact.

Private control requests and replies require the canonical product identity. This prevents a
same-version pre-rename process at an explicit data path from being treated as a compatible host.
The regression failed before the fix (successful attachment instead of refusal), then passed.
The shared MCP/table-manners contract remains unchanged at published v0.2.0.

PLAN now states that real Rascal entry, Squint offline entry, v0.2 integration and live session
inspection are merged. M4 still needs automatic discovery, production installation/registration,
unconfigured readiness/provider setup and complete first-install journeys. Sibling paths are
`../rascal` and `../squint` with no alternate checkout fallback.

## Verification

- Formatting and Clippy all targets/features with warnings denied passed.
- Required Rust suite: 72 passed, three opt-in probes ignored by default. All three probes also
  passed with isolated Rascal `27ef3b5` and Squint `3e6223a` testkit binaries, via direct MCP.
- TypeScript checks, 12 UI unit tests and two asset-manifest tests passed.
- Storybook build and all 74 Chromium/WebKit cases passed, including both real-tool journeys.
  An initial run found an ambiguous tool-picker selector after the earlier Tool setup addition;
  using its exact accessible name fixed it. The identity check also exposed an older session-test
  helper missing the product field; that helper now sends it. The complete suite was rerun.
  One intermediate run reported an empty draft in the WebKit setup test (73 passed, one failed).
  The unchanged test passed five isolated repetitions; the cause was not reproduced or declared
  fixed. Preserve this observation for follow-up if draft retention fails again.
- Two clean UI builds are byte-identical and match checked-in assets. Release build and renamed
  help/version commands passed. Native Safari was not rerun for the rename.
- The mock builder regenerated both standalone views. Case-insensitive tracked-content audit
  finds only the required former-name line in README. Git history and ignored build/cache files
  are not source and retain historical bytes; the checkout directory itself is not renamed here.
- Review found the live-host identity gap and the renamed-launcher prerequisite; both are fixed
  and the final diff is approved. No dependencies or storage schemas changed.

No remote was added; nothing was pushed or tagged. No sibling repository was edited. No preview
host is intentionally left running by this work.

## v0.3 readout, no adoption

Read table-manners branch `v0.3` at `91a26ff385a6679c348442dccc4b577b3cad6d74`, including
CONTRACT.md 3.1, 3.2, 4.5, 4.6 and the description/estimate schemas. The readiness, reserved
estimate operation, session configuration and render readiness direction supports the host.

One clarification is needed before paid interoperability: 4.5 permits `max_spend_usd`, while 5
defines action input schemas as `params_schema` plus profile inputs. Require every paid action
to declare that optional numeric argument in its `params_schema` and MCP input schema, with a
shared zero/positive-value rule. Otherwise a conforming closed schema can reject the host's cap
before the limits check. The current Rascal fixture has a closed schema without that property;
Squint declares it with an exclusive minimum of zero. Those fixtures are still earlier-version
evidence, so this is a request for the v0.3 conformance fixtures, not a claim that adoption is ready.

No v0.3 host implementation, schema pin change or sibling patch was made. Publication and matching
tool conformance fixtures remain prerequisites for an explicit adoption slice.
