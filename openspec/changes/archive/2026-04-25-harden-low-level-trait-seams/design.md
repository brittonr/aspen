## Context

Aspen has productized several crate families to `extraction-ready-in-workspace`: Redb Raft KV layers, coordination, protocol/wire, blob/castore/cache first slice, and branch/DAG first slice. The next blocker is not only dependency cleanup; it is that low-level crates still expose broad seams in some places:

- `aspen-traits::KeyValueStore` bundles read, write, delete, scan, and local scan behavior.
- `aspen-cache` has useful pure cache/signing logic, but KV-backed index/signing persistence still imports root `aspen-core` under `kv-index`.
- `aspen-blob` already splits blob traits, but `BlobAwareKeyValueStore` is tied to `IrohBlobStore` and root `aspen-core` instead of generic blob/KV capability traits.
- `aspen-redb-storage` implements OpenRaft traits directly; Aspen-owned storage port traits would make the storage behavior productizable and testable below the OpenRaft adapter.
- `aspen-kv-branch` and `aspen-commit-dag` depend directly on broad KV contracts where domain store traits would make branch/DAG persistence clearer.
- `aspen-time::TimeProvider` exists, but more low-level code can use provider injection rather than ambient time reads. `aspen-hlc` exposes concrete `uhlc` types where a logical-clock trait would let consumers avoid depending on the adapter type.

This change is a planning/specification change. It defines contracts and implementation tasks for future code work; it does not implement the trait splits.

## Goals / Non-Goals

**Goals:**

- Define a low-level trait seam contract for reusable crates.
- Establish the first implementation sequence from least-risk foundational seams upward.
- Require positive, negative, and compatibility evidence for each seam.
- Keep runtime coupling in adapter crates/modules/features.
- Preserve existing Aspen consumers through composite traits and compatibility rails during migration.

**Non-Goals:**

- Do not publish crates or mark anything `publishable from monorepo`.
- Do not remove existing public APIs in this spec-only change.
- Do not require all low-level seams to become `no_std`; use alloc/no-std only where already part of the crate contract.
- Do not make iroh, Redb, or OpenRaft disappear where they are the explicit backend purpose. Instead, classify them as adapter/backend surfaces.

## Decisions

### 1. Split KV by capability before changing product crates

**Choice:** Introduce narrow KV capability traits in `aspen-traits`, then keep a composite full-store trait for compatibility.

Initial target split:

- `KvRead`: single-key reads.
- `KvWrite`: write commands.
- `KvDelete`: delete commands.
- `KvScan`: prefix scans.
- `KvLocalScan` or `KvLocalRead`: stale/local state-machine reads.
- `KvTxn`: transaction/compare operations once existing transaction surfaces are ready.
- `KvLease`: lease/TTL operations if exposed as a public persistence capability.
- `KvStore`: composite for existing full KV behavior.

**Rationale:** Cache lookup, signing-key persistence, blob metadata, branch tips, and commit stores need different subsets. Forcing all of them to depend on a full cluster KV trait makes downstream fixtures and product crates pull unnecessary assumptions.

**Alternative rejected:** Keep `KeyValueStore` as the only trait and rely on documentation. That keeps APIs easier short-term but makes every seam as broad as cluster KV.

### 2. Productize `aspen-cache` through storage ports first

**Choice:** Start product code work with `aspen-cache` port traits after the KV split: `CacheLookup`, `CachePublish`, `CacheStatsRead`, `CacheStatsWrite`, and `SigningKeyStore`-style traits. KV-backed implementations become adapters over those ports.

**Rationale:** `aspen-cache` has a clear standalone product story and mostly pure default logic. Its remaining coupling is narrow enough to fix early and use as pattern for the rest.

**Alternative rejected:** Start with `aspen-blob`. Blob has high product value but its concrete iroh backend and replication concerns make it a larger adapter-boundary change.

### 3. Treat blob as capability traits plus concrete iroh adapter

**Choice:** Keep existing `BlobWrite`, `BlobRead`, `BlobQuery`, and transfer traits, but make `BlobAwareKeyValueStore` generic over blob read/write capabilities and split iroh transfer-specific concerns from non-transfer blob usage.

**Rationale:** The blob trait family is already close to the target. The main issue is adapter placement: KV offload should not require `IrohBlobStore`, and non-transfer users should not need peer transfer surfaces.

**Alternative rejected:** Make blob fully generic over all CAS backends immediately. The current product target is iroh-backed blob storage, so generic CAS abstraction beyond the current capability traits is deferred.

### 4. Add Aspen-owned Redb storage ports below OpenRaft

**Choice:** Define storage ports such as `KvStateReader`, `KvStateWriter`, `LeaseStateStore`, `RaftLogSegmentStore`, `SnapshotStore`, and `IntegrityChainStore` in or near `aspen-redb-storage`, then implement OpenRaft traits as adapters over those ports.

**Rationale:** `aspen-redb-storage` is product-shaped but currently largely expresses itself as an OpenRaft implementation. Ports make storage behavior testable without OpenRaft services and keep the single-fsync invariant visible.

**Alternative rejected:** Move OpenRaft trait impls unchanged and call that done. That would preserve behavior but would not create a clean product seam below OpenRaft.

### 5. Add branch/DAG domain stores after KV and cache seams

**Choice:** Introduce `CommitStore`, `BranchTipStore`, and possibly `BranchOverlayStore`/`ConflictDetector` abstractions after the foundational KV split is available. KV-backed branch/DAG persistence becomes an adapter.

**Rationale:** Branch/DAG semantics are domain-specific and should not expose broad KV details to callers. This work is less urgent than cache because branch/DAG already has no root runtime dependency, but it improves product shape.

**Alternative rejected:** Require every branch/DAG consumer to use `KeyValueStore` forever. That makes in-memory and non-KV storage adapters awkward and hides stale-tip/conflict semantics behind generic KV errors.

### 6. Push time and logical clock providers through low-level crates

**Choice:** Use `aspen-time::TimeProvider` wherever wall-clock reads are needed in reusable low-level code. Add a `LogicalClock` trait around HLC behavior when a consumer needs causal timestamp generation/observation without owning `uhlc`.

**Rationale:** Time injection improves deterministic tests and madsim friendliness. A logical-clock trait lets HLC remain an adapter while callers depend on a small contract.

**Alternative rejected:** Continue using ambient time helpers in low-level code. Aspen already treats `aspen-time` as the wall-clock boundary; extending that pattern is consistent.

### 7. Require three kinds of proof for every seam

**Choice:** Each seam must have:

1. Positive downstream-style fixture using the canonical narrow trait/port.
2. Negative boundary proof catching broad/root/runtime dependencies or invalid inputs.
3. Compatibility compile/test proof for existing Aspen consumers.

**Rationale:** Prior extraction work succeeded when proof was durable and negative. Positive-only tests miss dependency leaks and compatibility regressions.

**Alternative rejected:** Rely on ordinary unit tests. Unit tests do not prove dependency topology or downstream product shape.

## Risks / Trade-offs

- **Trait proliferation** → Mitigation: keep composite traits and prelude exports; split only around real consumer capability differences.
- **Large migration churn** → Mitigation: sequence from `aspen-traits` and `aspen-cache` first, then adapt blob/storage/branch/DAG incrementally.
- **Compatibility breakage** → Mitigation: keep composite traits and compatibility rails until all current consumers migrate.
- **Over-generalization** → Mitigation: require each new trait to have at least one concrete consumer and one adapter implementation in the same implementation slice.
- **Async trait object overhead** → Mitigation: favor generic bounds for hot paths and trait objects only at orchestration boundaries.
- **OpenRaft or iroh are legitimate product dependencies in some crates** → Mitigation: classify backend-purpose dependencies explicitly instead of banning them broadly.

## Migration Plan

1. Add capability traits to `aspen-traits` and keep the existing full-store behavior through a composite trait/compatibility alias.
2. Migrate `aspen-cache` persistence to `SigningKeyStore` and cache index capability traits; replace root `aspen-core` imports with leaf traits/types in KV adapters.
3. Make blob/KV offload generic over blob and KV capabilities; keep iroh transfer surfaces isolated.
4. Introduce Redb storage ports and adapt OpenRaft impls over them without changing storage invariants.
5. Add branch/DAG domain store traits and KV adapters.
6. Push `TimeProvider` and logical-clock traits through low-level consumers.
7. Update manifests, policy, downstream fixtures, negative boundary checks, and compatibility rails after each slice.

Rollback is per-slice: keep composite traits and old adapter paths until the replacement has evidence. If a slice fails compatibility rails, revert that slice without blocking the other planned seams.

## Open Questions

- Should the full compatibility trait keep the name `KeyValueStore`, or should a new `KvStore` composite be introduced with `KeyValueStore` retained as alias/re-export during migration?
- Should Redb storage ports live in `aspen-redb-storage` or a lower `aspen-storage-ports` crate once more backends exist?
- Should `LogicalClock::Timestamp` be an associated type wrapping `SerializableTimestamp`, or should Aspen define a stable leaf timestamp type before exposing the trait broadly?
- Which cache port names become public semver contracts once publication policy is decided?
