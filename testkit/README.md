# Independent offline peer

`sample-lab` is Wafflehouse's owned test tool. It is explicitly illustrative; it does not stand
in for Rascal's repair operations or Squint's judgments. Its wire implementation does not use
rmcp or Wafflehouse's coordinator/client. It may use the published contract's data/schema crate.

The initial fixtures target published `table-manners =0.1.0`. `fixtures.rs` builds deterministic
description/result documents. The owned PNG is `ui/tests/fixtures/sample.png`, a generated RGB
gradient; its byte fingerprint is computed, not copied from another tool's output. The fixture
input identity is a documented placeholder until the wire peer computes actual request identity.
The completed schema fixture deliberately has no outputs; wire tests will verify actual image
outputs. Future-version declarations are negative scenarios, never claims of v0.2 conformance.

Cancellation follows CONTRACT.md sections 2.1 and 4.2: status cancelled, null error, spend to date,
and no MCP result for a cancelled request. The shape was cross-checked against
`table-manners/fixtures/squint/run-cancelled.json` at `8e4d0c3`; it is not a Rascal fixture.
Schema retrieval follows `table-manners/tests/fixtures.rs`, resolves embedded URNs only, and
never retrieves arbitrary network references.

The host's production version/dispatch guard is implemented with its client in F10/F11; these
fixture tests do not substitute for that execution boundary.
