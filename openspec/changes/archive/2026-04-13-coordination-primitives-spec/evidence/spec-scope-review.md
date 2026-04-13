# Coordination spec scope review

## Umbrella vs dedicated lock-set domain

Reviewed together:

- `openspec/specs/coordination/spec.md`
- `openspec/specs/distributed-lockset/spec.md`
- `openspec/changes/coordination-primitives-spec/specs/coordination/spec.md`

Decision: keep `coordination` as umbrella domain and keep `distributed-lockset` as detailed atomic multi-resource contract.

Reason:

- umbrella spec now catalogs primitive families and shared RPC semantics
- lock-set spec still owns canonical ordering, bounded member count, per-resource fencing, and atomic multi-resource acquire/release behavior
- umbrella delta references lock sets only at catalog / RPC-contract level and does not copy those detailed invariants

## Worker coordination placement

Decision: keep `DistributedWorkerCoordinator` under umbrella `coordination` domain for now.

Reason:

- worker coordination lives in `crates/aspen-coordination/`
- proposal/design scope is umbrella primitive coverage, not backend reorganization
- current behavior is still coordination-facing: worker registration, liveness, capacity-bounded assignment, bounded work stealing
- if worker orchestration grows its own dedicated external contract later, it can split into a focused spec without changing this umbrella catalog
