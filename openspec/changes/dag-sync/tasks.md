# DAG Sync — Tasks

## Phase 1: Core Crate (`aspen-dag`)

- [x] Create `crates/aspen-dag/` with Cargo.toml, feature flags, and workspace integration ✅ 15m
- [x] Implement `DagTraversal` trait with `next()`, `db_mut()`, `roots()` methods ✅ 10m
- [x] Implement `FullTraversal` (depth-first pre-order with visited set and stack) ✅ 20m
- [x] Implement `SequenceTraversal` (fixed sequence of hashes) ✅ 5m
- [x] Implement `Filtered<T, F>` combinator for composable traversal filters ✅ 5m
- [x] Implement `Bounded<T>` combinator for depth/count limits ✅ 5m
- [x] Implement `LinkExtractor` trait for pluggable child-hash extraction ✅ 10m
- [x] Add `constants.rs` with Tiger Style bounds: `MAX_DAG_TRAVERSAL_DEPTH`, `MAX_VISITED_SET_SIZE`, `MAX_DAG_SYNC_REQUEST_SIZE`, `MAX_DAG_SYNC_TRANSFER_SIZE` ✅ 5m
- [x] Add `verified/traversal.rs` with pure functions: `should_visit`, `is_traversal_bounded`, `compute_visited_set_size` ✅ 10m
- [x] Write unit tests for `FullTraversal` with known DAG structures ✅ 15m
- [x] Write unit tests for `Filtered` and `Bounded` combinators ✅ 5m
- [x] Write property tests (proptest) for traversal determinism: same DAG + same config = same sequence ✅ 10m

## Phase 2: Wire Protocol

- [x] Define `DagSyncRequest` type: root hash, traversal options, known heads, inline predicate ✅ 10m
- [x] Define `TraversalOpts` enum: `Full`, `Sequence`, with filter and order options ✅ 5m
- [x] Define `InlinePolicy` enum: `All`, `SizeThreshold(u32)`, `MetadataOnly`, `None` ✅ 5m
- [x] Define `SyncResponseHeader` frame type (33-byte fixed: discriminator + BLAKE3 hash) ✅ 10m
- [x] Implement postcard serialization for all protocol types with roundtrip tests ✅ 10m
- [x] Implement `write_sync_response`: sender-side traversal → framed stream (`send_sync`) ✅ 15m
- [x] Implement `handle_sync_response`: receiver-side traversal + incremental insert (`recv_sync`) ✅ 15m
- [x] Add `DAG_SYNC_ALPN` constant to `aspen-transport` ✅ 2m
- [x] Write integration test: full sync of an in-memory DAG between two endpoints ✅ 15m
- [x] Write integration test: partial sync with known heads (only new objects transferred) ✅ 5m

## Phase 3: Forge Integration

- [x] Implement `GitLinkExtractor` for Forge's Git object types (commit→tree+parents, tree→entries, tag→target, blob→empty) ✅ 15m
- [x] Implement `CobLinkExtractor` for COB change parent references ✅ 10m
- [x] Add `DagSyncProtocolHandler` implementing `iroh::protocol::ProtocolHandler` ✅ 20m
- [x] Add DAG sync planning methods to `SyncService` (`plan_git_sync`, `plan_cob_sync`, `build_sync_request`) ✅ 20m
- [x] Add `build_stem_sync_request` / `build_leaf_sync_request` for two-phase sync ✅ 10m
- [x] Implement `known_heads` derivation from local branch refs (`known_heads_from_refs`) ✅ 5m
- [x] Add stem/leaf filter definitions (`ForgeNodeType`, `is_blob_node` / `is_stem_node`) ✅ 10m
- [x] Wire up ALPN handler: `RouterBuilder::dag_sync()` in aspen-cluster ✅ 5m
- [x] Implement `connect_dag_sync` client-side connection + request + receive ✅ 15m
- [x] Write e2e integration test: full sync of linear chain over real iroh QUIC ✅ 15m
- [x] Write e2e integration test: incremental sync with known heads ✅ 10m
- [x] Write e2e integration test: stem/leaf split with filter-based two-phase sync ✅ 10m

## Phase 4: Blob & Snix Integration

- [ ] Implement `DirectoryLinkExtractor` for snix Directory → sub-directory/blob links
- [ ] Add DAG traversal to blob replication repair scan for object-graph-aware discovery
- [ ] Write integration test: snix closure sync via DAG traversal
- [ ] Write integration test: blob repair scan discovers under-replicated subgraph

## Phase 5: Verification & Hardening

- [ ] Add Verus specs for traversal invariants: visited set monotonicity, depth boundedness, traversal determinism
- [ ] Add madsim deterministic test for DAG sync under network partitions
- [ ] Add madsim test for partial sync resumption after connection drop
- [ ] Benchmark: DAG sync throughput vs. per-object fetch for 5,000-object repo
- [ ] Benchmark: stem/leaf split with 3 peers vs. single-peer full sync
