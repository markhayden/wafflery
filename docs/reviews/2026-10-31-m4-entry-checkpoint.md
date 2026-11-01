# M4 checkpoint: explicit real-tool entry

Status: G01–G04 complete. This closes the explicit-entry slice in
[the M4 plan](../plans/m4-real-tool-entry.md), not automatic discovery or provider setup.
The owner also confirmed the M3 run/save/export checklist passed in native Safari on 2026-10-31.
That is owner verification; agent browser coverage below is Chromium and WebKit.

## Working behavior

Stopped-host registration accepts validated executable/argv/cwd/root descriptors, acquires the
host lifetime lock, and atomically appends private configuration. Exact duplicate records are
idempotent; conflicting IDs, malformed input and a running host are refused without replacing
existing configuration. Registration executes nothing. The host reserves one of the client's
32 allowed roots for its own materialized inputs, leaving at most 31 user roots.

`WAFFLEHOUSE_DATA_DIR` allows an isolated shared library through an unchanged H1 launcher:
`rascal playground` invokes exactly `wafflehouse --tool rascal`. The host owns attachment and
opening the browser. An unavailable selected tool stays unavailable with setup guidance; it
never silently selects another tool. Choosing a connected tool recovers the form.

`scripts/dev-rascal.ts` builds Wafflehouse, registers an explicitly selected Rascal binary in a
fresh owned workspace, and exercises the actual launcher. The generic UI runs an offline seam,
preserves evidence and journal, saves an independent example, exports image evidence, and reopens
history after restart. No Rascal-specific logic was added to production forms or result handling.

## Verification and review

- Rust formatting, all-target/all-feature clippy with warnings denied, and 62 Rust tests passed.
  The optional real-Rascal coordinator test also passed with zero reported spend and unchanged
  source bytes. It remains ignored in ordinary required checks.
- UI typecheck, 10 UI unit tests and two manifest tests passed. Browser verification built the
  app and Storybook. The final Chromium/WebKit suite passed all 58 cases, including two opt-in
  real-Rascal journeys and two unavailable-selection regressions.
- Embedded assets match their manifest; two clean builds are byte-identical. Release build passed.
  No dependencies changed; M3's source-package and dependency audit results remain applicable.
- Independent gpt-6-astra review found a root-limit mismatch: 32 registered user roots would
  become 33 at dispatch. Registration now reserves the host root; a boundary test verifies that
  31 user roots actually initialize through the coordinator. Final review found no remaining
  material issue in the production changes or strengthened real-tool test.
- An initial unknown-selector browser test navigated before bootstrap redemption in WebKit.
  Waiting for the connected tool first corrected the harness; the complete suite then passed.

Visual inspection caught a weak smoke fixture: a 160×100 image produced an empty mask because
Rascal's default band rounded below a pixel. A pixel-range assertion reproduced that failure
(maximum paint value 0). The owned 2048×1024 panorama now produces visible protected and painted
areas with default recipe parameters; both browser engines check those pixels. The output was
also inspected visually. Its cut-space mask has different coordinates from the original, so the
host correctly uses side-by-side viewing and disables wipe. This verifies mask preparation,
not AI repair quality. The fixture and reproduction are documented in the test kit.

The initial selected local Rascal binary reported version 0.1.0 and contract 0.1.0. During
verification the other session rebuilt that path against the 0.2 draft; Wafflehouse correctly
refused it. Final tests use a separate offline build of `git archive 9c52138` in owned temporary
storage. The original sibling working tree and its build products were not changed. The source
inventory distinguishes checkout revisions from executable provenance.

## Remaining work

The host still accepts exactly table-manners 0.1.0 and refuses paid actions. Paid execution is
authorized by the owner, but enabling it requires the implemented estimate/cap/session interface
and supported tool contract, not just permission or a provider key. Table-manners 0.2 is a draft
at this checkpoint. No paid call was made.

Automatic discovery, provider/readiness setup, location bindings, R3 operations, C2 portable native
replay, C3 tool pages and production installers remain gated as described in the plan. The real
workspace is a temporary development library. No production installation, sibling edit, push or
publishing was performed.
