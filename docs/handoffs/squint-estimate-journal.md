# Squint follow-up: estimate run has no readable journal

Resolved 2026-11-01: Squint fix `718eec7` is included in `3e6223a`; the isolated v0.2 testkit
probe now preserves complete estimate evidence, including an empty readable journal.
The original report below is retained as history.

Observed after verifying the fixes in [Squint #2](https://github.com/markhayden/squint/issues/2),
against an isolated `a0a2dde` testkit build. Discovery and scripted checks now pass.
This follow-up has not been posted to GitHub.

An MCP `estimate` with `use_case: "source-fidelity"`, candidate/source images, and the owned
offline config completes successfully with zero spend and a positive forecast. Its run.json is
readable, but reading `squint://run/<estimate-id>/journal` returns MCP `-32002` / no resource.
Wafflery saves the result and correctly marks contract evidence **incomplete**.

In `crates/squint-cli/src/serve/mod.rs`, `finished()` writes the estimate result, while
`resource(.../journal)` concatenates `run-<n>/journal.jsonl` and returns no resource when the
text is empty. An estimate has no decision journal. Shared v0.1 run resources advertise the
journal; the host should not guess which actions are exceptions.

Please provide a readable journal for existing estimate runs (an empty journal, if supported by
the contract, or a minimal estimate event), while keeping unknown run IDs unavailable. Test
through `resources/read`, including after restart. No paid provider is required.

Reproduction from Wafflery:

```sh
WAFFLERY_SQUINT_BIN=/absolute/path/to/testkit/squint \
  cargo test --locked --test squint_smoke real_squint_estimate -- --ignored --nocapture
```

That test intentionally still requires complete evidence; it fails at `a0a2dde`. The separate
`real_squint_scripted` probe passes. Do not weaken preservation checks to clear the estimate gate.
