# v0.3 execution checkpoints

The owner approved V01–V18 on 2026-11-01. Implement in PLAN order, with tests first, accurate docs,
short branches, the full matrix under the shared heavy-check lock, and local merges after review.
The coordinator scans and pushes; no sibling edits, tags or compatibility shims.
The browser correction at 51d7935 passed hosted run
[41358441245](https://github.com/markhayden/wafflery/actions/runs/41358441245); the CI gate is green.

## V01: independent fixtures before live cutover

- V01a: archive the eleven schemas from table-manners e503a238ad6cc833834891c2c2cece7152b08bfc
  with provenance/license; add owned descriptions, sessions, estimates and refusal fixtures,
  including two tool names, all readiness states and estimate refusal variants. Validate with
  an independent JSON Schema retriever; malformed readiness, absent positive-cap declarations
  and malformed estimates must fail. Keep the live host's exact v0.2 guard and paid guard.
- V01b: expose those fixtures and scripted estimate control over the existing independent
  line-delimited peer. Test initialize/discovery, session consistency, input refusals and no run
  allocation/spend. No host SDK or tool-private code in the peer. Describe/estimate only: future
  run behavior is added in the owning task, not claimed as implemented here.
- Each slice carries its tests and testkit docs. Validate the joined V01 checkpoint with the full
  matrix, review and then merge locally after hosted CI for the prior fix is green.

V02 uses the authorized temporary sibling path while developing, then switches to the published
exact crate before its hosted checkpoint. V01's archived schema input avoids changing the live
validator prematurely. Remove the archive in V02 when the dependency supplies the same schemas.

Rollback newest dependent commits first. V01 changes only disposable peer/test coverage; no real
library migration, provider calls or host dispatch behavior. Later rollback checkpoints remain
those in PLAN.md.

## V01 verification

Both test slices were observed failing before implementation: null fixture objects fail the
reviewed schema, and the old peer neither describes v0.3 nor implements estimate. The completed
fixtures and wire probes pass, including actual payload schema validation, before/after ledger
identity, zero spend/reservations, no runs/files, fresh serving sessions, reserved-name exclusion,
readiness/cap/budget estimates and invalid-argument refusals. The runtime still rejects v0.3.

Implementation and test coverage passed review. Under the shared heavy-check lock: formatting,
clippy, 77 Rust tests (three existing opt-in ignores), UI/script typecheck, 12 UI unit tests,
two asset-tooling tests, Storybook, byte-identical clean UI builds, release build and 70 browser
cases passed; four existing real-tool browser cases were skipped. Generated assets/lockfiles
remained unchanged. No provider calls, sibling edits, push or tags.

## V02: coordinated validation cutover

Use the authorized temporary path dependency at reviewed e503a23 during development; switch to
published `=0.3.0` before the final checkpoint. Keep MCP 2025-11-25 and the no-paid dispatch guard.
Accept only v0.3 live descriptions/results/sessions, require estimate discovery for paid actions,
remove cost.needs_env, and retain generic reading of older saved evidence. V03 owns the richer
readiness UX; V07 owns list-input editing and dispatch.

The approved larger-file exception covers this coordinated change. Enumerated sites:
Cargo.toml/Cargo.lock; src/tools/{mod,client,session}.rs; testkit/{main,fixtures,scenarios,
v03_control}.rs; tests/{mcp_client,contract_fixtures,v03_fixtures,v03_wire,attempt_storage,
tool_session_api,rascal_smoke,squint_smoke}.rs and tests/support/v03_schema.rs; UI api/{workspace,
tool-session}.ts, features/actions/{model,ActionForm.stories}.tsx/ts, workspace/{ToolSetup,
ToolSetup.stories}.tsx, affected action/session unit tests and tool-setup/real-tool browser tests;
README, ENGINEERING, integration-sources, foundation-runtime and testkit docs. Remove the generated
V01 schema archive/license when the dependency supplies it. Commit generated UI assets with the
UI type/copy change. Any additional affected site must be listed in the verification record.

Tests first: discover unconfigured v0.3 peers with arbitrary names, require reserved estimate
without exposing it as an action, reject live v0.2 and missing-cap/estimate declarations, preserve
v0.1/v0.2 saved evidence, and continue current offline execution. Review the schema/parser/fake
change together; run the full matrix under the shared lock. Roll back this coherent cutover
commit rather than retaining dual-version dispatch. No data migration is required.
