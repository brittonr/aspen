## Phase 1: Fabric architecture contract

- [x] [serial] Define the workload-neutral fabric boundary and mechanism-versus-semantics ownership rules. r[molten.fabric_boundary.fabric_identity] r[molten.fabric_boundary.mechanism_semantics_separation]
- [x] [serial] Define sandboxed plugin, system-extension, and application/workload tiers with explicit authority boundaries. r[molten.fabric_boundary.extension_tiers]
- [x] [parallel] Document the fabric architecture and update project-facing terminology without introducing database or protocol compatibility claims. r[molten.fabric_boundary.fabric_identity] r[molten.fabric_boundary.non_claims]

## Phase 2: Capability port registry

- [x] [serial] Add pure canonical fabric-port descriptor and registry models with stable identity, version, profile, schema, authority, resource, replay, and non-claim fields. r[molten.fabric_boundary.port_registry]
- [x] [parallel] Add positive fixtures for compatible unique ports and negative fixtures for unknown, duplicate, incompatible, silently substituted, or over-authorizing ports. r[molten.fabric_boundary.port_registry]

## Phase 3: Fabric sufficiency and evidence scope

- [x] [serial] Define reference-system capability matrices for a transactional key-value service, replicated log, and distributed scheduler. r[molten.fabric_boundary.reference_system_exit_criteria]
- [x] [parallel] Add evidence-granularity guidance and tests preventing per-operation receipts from being required below declared semantic or trust boundaries. r[molten.fabric_boundary.evidence_granularity]
- [x] [parallel] Add explicit non-claim fixtures for global consensus, global ordering, database correctness, protocol compatibility, and production readiness. r[molten.fabric_boundary.non_claims]

## Phase 4: Validation

- [x] [serial] Run focused fabric descriptor, tier, registry, reference-matrix, and negative-boundary tests. r[molten.fabric_boundary.final_validation]
- [x] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.fabric_boundary.final_validation]

## Validation evidence

- Baseline: `nix develop -c cargo test fabric --lib` passed with no pre-existing fabric tests; `nix develop -c cargo test -p molten-core` passed 26 tests.
- Focused: `nix develop -c cargo test -p molten-core fabric` passed 16 positive and negative pure-core tests; `nix develop -c cargo test fabric --lib` passed 5 canonical Preserves and denial tests.
- Workspace: `nix develop -c cargo test --workspace` and `nix develop -c cargo clippy --workspace --all-targets -- -D warnings` passed.
- Lifecycle: Cairn validation passed for 24 active changes and 59 specs; proposal, design, and tasks gates passed before sync.
- Traceability: all eight `molten.fabric_boundary.*` ids have implementation and verification markers and are absent from the missing/dangling sets. The repository-wide coverage rail remains pre-existingly red with 2,263 accepted requirements versus 256 referenced requirements and one unrelated dangling `molten.testing.multinode.three_node_membership_negatives` marker.
