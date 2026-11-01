# Table-manners v0.2 integration

Implemented 2026-11-01. See [the checkpoint](../reviews/2026-11-01-table-manners-v02.md).

Use published table-manners =0.2.0 (tag 3c8b55c); keep MCP 2025-11-25 and H1 unchanged.

1. Migrate the live version gate and independent peer to v0.2; old saved attempts remain readable without rewriting them. Preserve the may_spend:false dispatch restriction.
2. Record validated server session observations and send stable client_run_id metadata. Reject conflicting run identities. Preserve structured progress association without parsing messages.
3. Reconcile cancellation through an associated run resource, with bounded lookup and no automatic resubmission. Missing evidence stays unconfirmed.
4. Verify published schemas, adversarial association/session tests, normal/cancelled evidence, UI gates, and isolated committed Rascal/Squint binaries. Re-run the Squint estimate journal probe.
5. Update engineering/status documentation, independent review, and commit the verified integration.

Non-goals: paid-provider setup, budget configuration, automatic restart/replay, contract changes, typed operations, final navigation hierarchy. Test roots are disposable; personal libraries and sibling working trees are untouched.

## Commit boundaries

The exact dependency/schema fixture adoption is separate from host integration. Session and
association enforcement must land with the peer and cancellation regressions as one coherent
host change; this cross-cutting slice exceeds the approximate five-file budget. The UI gate and
inspector, generated assets, and documentation follow separately on the same branch. Merge only
the complete reviewed stack; the dependency-only intermediate commit is not an integration release.
