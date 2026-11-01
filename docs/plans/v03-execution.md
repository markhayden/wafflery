# v0.3 execution checkpoints

The owner approved V01–V18 on 2026-11-01. Implement in PLAN order, with tests first, accurate docs,
short branches, the full matrix under the shared heavy-check lock, and local merges after review.
The coordinator scans and pushes; no sibling edits, tags or compatibility shims.
The browser correction at 51d7935 is awaiting hosted confirmation before the next merge.

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
