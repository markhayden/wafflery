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

## Run the wire checks

`cargo test --locked --test fake_wire` builds the `sample-lab` example, then starts it in
temporary directories using an independently written line-delimited client. The probe checks
2025-11-25 initialization, roots, tool pagination, both path and inline image calls, expected
fingerprint refusal, output digests, multiple run resources, cancellation and EOF shutdown.

The example takes `<workspace> [scenario]`. `normal` and `slow` execute the offline image
operation. `paid`, `unsupported-contract`, `wrong-protocol` and `malformed` are negative test
scenarios, not full conformance claims; none makes a paid or remote call. Only the host's owned
test descriptor will opt into this executable. No real tool is discovered or installed by it.

## Optional real Rascal smoke

Select a local binary explicitly (never discovered or installed automatically):

```sh
WAFFLEHOUSE_RASCAL_BIN=/absolute/path/to/rascal cargo test --locked --test rascal_smoke -- --ignored --nocapture
```

This uses the same host coordinator, fresh no-paid check, exact contract-version allowlist,
image fingerprints and evidence capture as the fake. The child inherits no environment,
receives no config or filler, and gets temporary workspace/run directories. It checks a real
`seam` result, zero spend, verified outputs/journal and unchanged original bytes. `mask_rect`
requires a location binding, which is outside the foundation's image-and-parameters form.

The seam needs explicit equirectangular/wrap image coordinates. The host's optional `frames`
field records that shared image-reference metadata alongside ordered asset bindings. Ordinary
images default to flat/bounded. This metadata is validated using the published image-reference
schema; the host never interprets a tool's private projection parameter. The smoke explicitly
sets Rascal's declared parameter as well. A flat seam can complete without producing an image,
so that alone is not a successful output smoke.

Missing binaries or unsupported contract versions print an explicit `SKIPPED` reason. Other
failures remain failures; do not loosen the version, spend or fingerprint guards to pass them.

## Optional real Rascal browser journey

```sh
WAFFLEHOUSE_RASCAL_BIN=/absolute/path/to/rascal bun run --cwd ui test:browser
```

The optional Chromium/WebKit journey uses explicit registration and the same generic UI as any
tool. It checks a real seam mask, saved inputs, recorded call, image evidence download, independent
example loading, and history/output preservation across a host restart. Without the environment
variable those two cases are reported skipped. Choose a stable contract 0.1.0 executable; rebuilding
a selected binary against a newer draft during the suite will correctly make it unavailable.

`ui/tests/fixtures/panorama.png` is an owned, procedurally generated 2048×1024 RGBA gradient:
`R = 60 + floor(80*x/2048)`, `G = 80 + floor(100*y/1024)`, `B = 110`, `A = 255`.
Its unequal left/right edges create a deterministic wrap seam. The browser checks actual mask
pixels contain both protected black and paint values above 127. The small 160×100 fake-tool
fixture rounds Rascal's default band below a pixel, so an image-presence assertion alone would
miss an empty mask. The panorama uses default recipe parameters. Source and cut-space mask have
different coordinates; the test requires side-by-side viewing and a disabled wipe, not invented
alignment. With no filler this verifies mask preparation, not a repaired image or model quality.
