# Squint integration probe: discovery and scripted cost metadata

2026-10-31. Please review and fix these in Squint; Wafflehouse has not changed sibling files.
Evidence uses committed Squint `fcde474`, copied with `git archive` into owned temporary storage
and built `--offline --locked --bin squint --features testkit`. The active Squint checkout has
uncommitted v0.2 work, which was neither built nor treated as released. Wafflehouse still pins
published table-manners 0.1.0. H1 remains unchanged.

## 1. Discovery cannot compile the advertised input schema

Wafflehouse initializes MCP 2025-11-25, then validates each complete `tools/list` input schema.
It fails before calling `describe`:

```text
Invalid schema: Pointer '/$defs/Kind' does not exist
```

In `crates/squint-core/src/contract.rs`, `obligation_schema()` produces `$defs.Kind` and
`#/$defs/Kind`. `params_schema()` embeds that fragment unchanged. For `source_fidelity`:

```text
definitions: /properties/requirements/items/$defs/Kind
reference:   /properties/requirements/items/properties/kind/$ref = #/$defs/Kind
root $defs:  absent
```

The reference resolves from the document root, not the nested fragment. Please make both the
complete action `params_schema` and the complete MCP `inputSchema` independently compilable,
including every use-case schema and estimate. Test the whole published documents with a JSON
Schema validator and an independent MCP client; validating only the obligation fragment misses
this failure. This is an implementation defect, not a request to relax the shared contract.

## 2. Scripted checks still advertise possible spend

With `serve --provider-script <fixture>` and the owned fake-provider config, all five check
actions declare `cost.may_spend: true`; only `estimate` declares false. Wafflehouse correctly
refuses those checks under its current execution policy. A scripted provider existing is not
enough for a generic host to infer that a call is free.

Please expose truthful capabilities for the actual configured executor. If the entire testkit
path, including enabled tool packs, guarantees no paid work and needs no credentials, declare
`may_spend: false` and the corresponding credential requirements. Otherwise retain the honest
paid declaration and supply a guaranteed-offline test configuration. Keep real paid providers
declared paid. Do not add a Wafflehouse-specific bypass or rely on parsing `--provider-script`.

## Reproduce from Wafflehouse

```sh
WAFFLEHOUSE_SQUINT_BIN=/absolute/path/to/stable/testkit/squint \
  cargo test --locked --test squint_smoke -- --ignored --nocapture
```

`testkit/squint/config.toml` and `held.json` are owned fixtures. The test copies them into an
isolated directory, clears the MCP child environment, supplies no keys and preserves original
image bytes. At `fcde474` it fails during discovery; this is recorded as blocked, not passed.
After the schema fix it is intended to verify paid-check refusal and a zero-spend estimate's
saved evidence. If scripted actions become truthfully free, update that probe to run a scripted
check; generic paid-refusal coverage already exists in Wafflehouse's required tests.

Separately verified: CLI `estimate --use-case source-fidelity` with the owned candidate/source
image and catalog model `claude-haiku-5-5` returns an envelope of `0.6251859` USD, fits a 2 USD
cap, and invokes no provider. That envelope is a forecast, not money spent. This proves the
offline estimator works, not that the MCP/UI path works.

## Return / remaining gates

Return a committed revision, supported contract version and focused verification results.
Prefer a compatible fix on the 0.1 path if practical; a fix only on 0.2 also requires Wafflehouse's
separate contract adoption. Do not tag or publish a contract merely to satisfy this probe.

List-valued `references` is a later shared-contract gate: the MCP schema accepts an array while
0.1's image slot metadata does not declare cardinality. Wafflehouse will not inspect `x-squint`
to invent it. The estimate probe uses candidate/source only. Config-free readiness, C2 exports
and C3 report readiness remain in the broader [Squint handoff](squint.md).
