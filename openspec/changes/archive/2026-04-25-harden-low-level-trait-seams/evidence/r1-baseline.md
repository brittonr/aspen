# Baseline Evidence: Low-Level Trait Seams
Generated: 2026-04-25T19:47:00-04:00
Commit: 1172654dd

## Summary

### aspen-traits
- KeyValueStore bundles 5 methods: write, read, delete, scan, scan_local
- Clean leaf deps: aspen-cluster-types, aspen-kv-types, async-trait
- CoordinationBackend trait returns Arc<dyn KeyValueStore>

### aspen-cache
- KV persistence behind kv-index feature (imports aspen-core shell crate)
- signing.rs and index.rs import aspen_core::KeyValueStore
- Pure logic (narinfo, NAR, signing) has no root deps

### aspen-blob
- Unconditional hard dep on aspen-core (shell crate)
- BlobAwareKeyValueStore hardcoded to IrohBlobStore (lines 49, 61, 73, 87)
- Blob traits already ISP-split: BlobWrite, BlobRead, BlobTransfer, BlobQuery
- Replication feature gates aspen-client-api

### aspen-redb-storage
- OpenRaft traits implemented directly: RaftLogReader, RaftLogStorage, RaftSnapshotBuilder, RaftStateMachine
- No Aspen-owned storage port traits below OpenRaft
- 24 source files across raft_storage/ and verified/ modules

### aspen-kv-branch
- Clean leaf deps: aspen-traits, aspen-kv-types, aspen-constants
- BranchOverlay<S: KeyValueStore> uses full trait bound
- 15 KeyValueStore references in overlay.rs alone

### aspen-commit-dag
- Clean leaf deps: aspen-kv-types, aspen-traits, aspen-constants
- 10 functions take &dyn KeyValueStore (store.rs, gc.rs, fork.rs)
- Uses full write+read+scan+delete from KeyValueStore

### aspen-time
- TimeProvider trait with now_unix_ms() and now_unix_secs()
- SystemTimeProvider (production) and SimulatedTimeProvider (simulation feature)

### aspen-hlc
- Re-exports raw uhlc types: HLC, ID, NTP64, HlcTimestamp
- SerializableTimestamp wrapper for serde
- aspen-core re-exports the same uhlc types via aspen_hlc
- aspen-jobs uses concrete HLC, HlcTimestamp, ID, NTP64 types

## Consumers of broad KeyValueStore

| Consumer | Usage Pattern |
|----------|--------------|
| aspen-commit-dag | &dyn KeyValueStore for store/load/gc |
| aspen-kv-branch | BranchOverlay<S: KeyValueStore> |
| aspen-cache (kv-index) | KvCacheIndex<KV: KeyValueStore> |
| aspen-blob | BlobAwareKeyValueStore<KV: KeyValueStore> |
| aspen-rpc-core | Arc<dyn KeyValueStore> in context structs |
| aspen-ci-executor-shell | Arc<dyn KeyValueStore> in config/common |
| aspen-raft-kv | Arc<dyn KeyValueStore> backend |

## Key Findings

1. KeyValueStore is monolithic: read+write+delete+scan+scan_local in one trait
2. aspen-blob has unnecessary hard dep on aspen-core shell crate
3. BlobAwareKeyValueStore hardcoded to IrohBlobStore, not generic over blob capabilities
4. aspen-commit-dag uses full KeyValueStore but only needs read+write+scan+delete
5. aspen-redb-storage has no ports below OpenRaft adapters
6. HLC consumers depend on concrete uhlc types rather than a logical clock trait
7. aspen-cache kv-index imports aspen-core instead of aspen-traits for KeyValueStore
