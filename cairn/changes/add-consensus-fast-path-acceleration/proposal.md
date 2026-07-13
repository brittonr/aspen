## Why

A live Molten consistency service may eventually benefit from lower remote-write latency in geo-distributed deployments. Jetpack demonstrates that a crash-fault fast path can be composed with an existing protocol while retaining the original path as fallback, but it also adds safety-critical conflict classification, view-recovery ordering, resource overhead, reduced fast-path availability, and environment-specific performance trade-offs.

Molten must not implement that optimization before its base live Raft profile, transport, durability, recovery, and operational evidence exist. It also must not hide the composition inside a generic engine name or transfer external model-checking and benchmark claims. A separate blocked Cairn package records the exact prerequisite, interface, evidence, and promotion boundaries for a future optional acceleration profile.

## What Changes

- Add an explicit optional consensus acceleration profile that composes with an exact admitted base-engine implementation and preserves the base path unchanged when disabled or unavailable.
- Bind a versioned extension-owned conflict contract, same-view fast quorum policy, proposer promises, recovery-marker protocol, canonical command identity, and duplicate suppression to the composite implementation profile.
- Add typed adaptive path-selection policy and telemetry without inheriting unreviewed paper thresholds or exposing engine internals to extensions.
- Add deterministic simulation, multi-process live, failure-recovery, resource, and environment-scoped performance admission before any production enablement.
- Keep the initial profile crash-fault, static-membership, non-transactional, opt-in, and default-off; deny Byzantine, dynamic-membership, interactive-transaction, and unsupported read/write semantics.
- Emit bounded semantic evidence rather than per-message receipts and preserve original-path performance and availability as explicit acceptance conditions.

## Impact

- **Files**: acceleration descriptors and Nickel contracts, engine registry and group manifest bindings, conflict-contract port, client routing policy, fast-path service shell, recovery state, normalized evidence, operator status, benchmark profiles, and `cairn/specs/consensus/spec.md`.
- **Testing**: model prerequisite checks, descriptor compatibility, conflict/fallback behavior, live fast/original convergence, view changes and recovery, partitions and crashes, three-node and five-node availability, adaptive backoff, original-path equivalence, resource bounds, and geo-distributed performance evidence.
- **Safety**: the change remains blocked until `model-consensus-fast-path-hazards` is archived and an exact live base Raft implementation is production-admitted. External Jetpack proofs, code, and benchmarks do not transfer to Molten. Fast-path success does not prove extension semantics, cross-group transactions, Byzantine tolerance, global ordering, or release readiness.
