## Context

Molten needs one efficient representation for branchable ordered semantic state. The map is a mechanism beneath the semantic-state typed root. It does not own the complete world, branch graph, authority, merge policy, effects, or storage transactions.

Dolt Prolly Trees and the pinned DoltLite cohort are design and behavior references. Molten uses its own Rust implementation, canonical format, BLAKE3 domains, bounds, tests, proofs, and claim boundary.

## Decisions

### Decision: Limit the map to keyed semantic state

**Choice:** `prolly-semantic-map-v1` represents canonical ordered semantic key and value bytes. Other world roots keep their own representations and policies.

**Rationale:** Shared structure helps keyed state. It would erase important semantics if imposed on snapshots, effects, authority, or executable extents.

### Decision: Keep the deterministic core separate from storage effects

**Choice:** The pure core validates profiles and nodes, plans reads, applies admitted edits to supplied nodes, derives new immutable nodes, compares maps, and computes reachability and GC plans.

The shell loads blocks, stages new blocks, publishes them transactionally, performs compare-and-advance, reconciles unknown outcomes, and executes admitted retention plans.

**Rationale:** The map defines deterministic structure. Storage, concurrency, and deletion remain external effects.

### Decision: Bind every structural input into the profile identity

**Choice:** A typed Nickel profile declares canonical key and value codecs, comparison, node format, boundary domain and seed, encoded-size accounting, minimum, target, and forced maximum chunk bounds, fanout bounds, and BLAKE3 domains.

The canonical profile projection receives a domain-separated BLAKE3 identity. Nodes and roots bind that profile identity.

**Rationale:** Equal logical state is not enough for equal roots when two implementations use different structural rules.

### Decision: Use bounded key-derived split decisions

**Choice:** Boundary decisions use canonical key bytes, current encoded node size, and the versioned profile. Values do not supply entropy for the boundary decision. Size-aware probability guides normal splits. The forced maximum encoded byte bound always terminates a node.

**Rationale:** Key-derived boundaries preserve locality when values change. Size accounting and a forced maximum prevent unbounded nodes under chosen keys or large values.

### Decision: Require history-independent roots

**Choice:** Any admitted mutation sequence that produces the same canonical ordered key-value map under one profile must produce the same root.

Tests cover insert, update, delete, batch, replay, and compaction histories. A repair or compaction operation cannot silently choose a different canonical tree.

**Rationale:** Branch comparison and deduplication need identity to represent state, not mutation history.

### Decision: Make diff complete and policy-neutral

**Choice:** Diff aligns key ranges and skips subtrees with equal canonical identities. It reports complete added, removed, and modified semantic entries under explicit bounds and hash assumptions.

The map does not decide whether two entries can merge. Molten typed merge policy consumes the diff.

**Rationale:** Storage difference and semantic merge authority are separate capabilities.

### Decision: Plan reachability but do not delete

**Choice:** The core computes reachable and candidate-unreachable identities from supplied roots, pins, and a complete observed graph. An incomplete graph yields an incomplete plan that cannot authorize deletion.

The retention shell revalidates roots, pins, generation facts, and policy before any destructive effect.

**Rationale:** Reachability is not deletion authority.

### Decision: Use local proof and independent behavior evidence

**Choice:** Trellis owns reusable local proof obligations for sorted uniqueness, search containment, boundary determinism, edit preservation, diff soundness, and reachability under explicit BLAKE3 assumptions.

Property tests and bounded model tests remain required. The DoltLite oracle provides independent behavior observations, not proof or format parity.

**Rationale:** Each evidence form has a different claim boundary.

### Decision: Delay extraction

**Choice:** The pilot stays in Molten. The benchmark classifier may recommend `evaluate-shared-component` only after two credible consumers require the same map semantics and evidence boundaries.

**Rationale:** Choregraph history and transfer chunks are not automatic second consumers. Premature extraction would create a generic storage monolith.

## Verification strategy

Positive tests cover empty and non-empty maps, point and range reads, all edit classes, equal-state history independence, structural sharing, localized diffs, closure enumeration, stable profile identities, restart, and bounded GC planning.

Negative tests cover malformed and noncanonical nodes, unsorted or duplicate keys, wrong profile, child range overlap, missing children, oversized values, chosen-key split pressure, forced-bound failure, tampered bytes, stale publication, unknown commit outcome, incomplete graph, active pins, and semantic-merge overreach.

## Rollout

1. Land typed profiles, canonical fixtures, and the pure node validator.
2. Add read and full rebuild behavior before incremental edits.
3. Add incremental edit and complete diff under differential tests.
4. Add block-store shell publication and crash recovery.
5. Add reachability, retention integration, and benchmark classification.
6. Evaluate extraction only after the declared evidence gate passes.

## Claim boundary

A passing pilot establishes bounded behavior for the exact profile, fixtures, adapters, and hash assumptions. It does not prove collision impossibility, database correctness, authority, effect safety, replication correctness, GC execution safety, universal performance, or release eligibility.
