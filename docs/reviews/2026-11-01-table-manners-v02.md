# Table-manners v0.2 integration checkpoint

Implemented 2026-11-01 on `feat/table-manners-v02`, based on `f9676f9`.

## Result

Live execution accepts exactly published `table-manners =0.2.0`, tag
`3c8b55c70f03581757549f67d04b85dc393ad88d`. MCP stays 2025-11-25 and H1 is unchanged.
Older saved attempts remain readable; no personal library or sibling working tree was changed.
The may_spend:false dispatch restriction remains enforced even though the owner has authorized
paid testing. Provider readiness, input-bound estimates and per-call caps still need implementation.

The host records the server ledger before dispatch and after preservation, sends stable client
correlation IDs, records rmcp's assigned progress token at the write boundary, and rejects mismatched
result/progress identities. The shared inspector exposes Session evidence. A reset session cannot
silently replenish allowance and continue execution. This ledger is not a provider account balance.

After cancellation, a bounded lookup can preserve the associated terminal resource and journal.
Resource recovery is recorded separately from an MCP call response. Missing evidence stays
unconfirmed; contradictory identities, duplicate matches across pages and incomplete pagination
cannot release the reservation. There is no automatic retry, replay, import or child restart.

## Verification

- Required Rust suite: 70 passed, zero failed; three optional executable probes ignored by default.
- Optional probes on isolated committed builds: Rascal `27ef3b5` offline seam, Squint `3e6223a`
  scripted source-fidelity, and Squint estimate: all three passed. No keys or paid provider calls.
- Squint's previously missing estimate journal is resolved by included fix `718eec7`; the
  complete-evidence assertion passes unchanged.
- UI typecheck and 10 unit tests passed. Storybook builds in the browser suite.
- Chromium/WebKit suite: 64 passed on the full run, two initial Chromium timeouts. Trace shows
  both waiting nearly 12 seconds for Run-button stability at the same time. Both passed unchanged
  in the isolated Chromium rerun (2.0s and 2.3s). All 66 cases are covered; this is not a claim
  of one uninterrupted 66-green run. Native Safari was not rerun; the owner's earlier M3 pass
  remains the only native Safari verification.
- Clippy all targets/features with warnings denied, formatting/diff checks, release build passed.
- Two clean UI builds are byte-identical and match the embedded asset manifest.
- Inspected the browser screenshot of the cancelled result drawer; the Session tab fits alongside
  existing evidence tabs. Final workspace navigation hierarchy remains separate UI work.

## Independent review

Reviewer `/root/m2_review` requested two corrections: scan all bounded history pages before
accepting a list-derived match, and keep contradictions terminal instead of retrying them away.
Both were fixed. The reviewer then caught an early-return window in the negative tests; they now
wait for a recovery-specific failure notice before asserting the unconfirmed reservation.
The final review approved these changes; reviewer did not independently rerun builds/tests.

## Remaining work

Generic provider readiness/configuration, estimate/cap setup and paid dispatch remain open.
Automatic discovery/install, typed Operations, native replay/export and embedded tool viewers
retain their existing interface gates. v0.2 still has scalar image slots; list-slot proposals
were not adopted accidentally. No contract changes or external publication were made.
