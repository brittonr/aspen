# Implementation Prompt: Close Feature Gaps with FoundationDB and etcd

## Context

You are working on Aspen, a distributed key-value store written in Rust. The codebase has ~21,000 lines of production code with 350+ tests. It uses:

- **openraft** (vendored v0.10.0) for Raft consensus
- **redb** for append-only Raft log storage
- **SQLite** for ACID state machine storage
- **Iroh** for P2P networking (QUIC, NAT traversal)
- **madsim** for deterministic simulation testing

A gap analysis identified three HIGH severity issues compared to FoundationDB and etcd. Your task is to implement solutions for these gaps.

## Reference Documents

Read these files before starting:

- `.claude/progress-2025-12-22.md` - Detailed implementation plan with phases
- `.claude/foundationdb-etcd-gap-analysis-2025-12-22.md` - Full gap analysis

## Implementation Tracks

Work on **P0 and P1 in parallel**. P2 is blocked until P0 completes.

---

## Track A: P0 - Transaction Model Enhancement

### Problem

Version tracking is incomplete. Only `apply_transaction_put()` increments versions. Regular writes (`apply_set`, `apply_set_multi`, `apply_compare_and_swap`, `apply_conditional_batch`) bypass versioning entirely. There's no OCC conflict detection.

### Phase 1.1: Complete Version Tracking

**Files to modify:**

- `src/raft/storage_sqlite.rs` - State machine apply functions

**Tasks:**

1. Modify `apply_set()` (~line 2087):

   ```rust
   // Before INSERT OR REPLACE, query existing version:
   let (new_version, create_rev) = match conn.query_row(
       "SELECT version, create_revision FROM state_machine_kv WHERE key = ?",
       params![key],
       |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
   ) {
       Ok((version, create_rev)) => (version + 1, create_rev),
       Err(_) => (1, log_index as i64),
   };
   // Then include version, create_revision, mod_revision in INSERT
   ```

2. Apply same pattern to:
   - `apply_set_multi()` (~line 2118)
   - `apply_compare_and_swap()` (~line 2223)
   - `apply_conditional_batch()` (~line 2396)

3. Add tests in `tests/` verifying version increments for each write path

**Success criteria:**

- All writes update `version`, `create_revision`, `mod_revision`
- `get_with_revision()` returns correct metadata after any write type
- Existing tests pass

### Phase 1.2: Read/Write Set Capture

**Files to modify:**

- `src/api/mod.rs` - Add new WriteCommand variant

**Tasks:**

1. Add new enum variant:

   ```rust
   WriteCommand::OptimisticTransaction {
       read_set: Vec<(String, i64)>,    // (key, expected_version)
       write_set: Vec<WriteOp>,          // operations to apply
   }
   ```

2. Add `WriteOp` enum for transaction operations:

   ```rust
   pub enum WriteOp {
       Set { key: String, value: String },
       Delete { key: String },
   }
   ```

3. Implement `apply_optimistic_transaction()` in storage_sqlite.rs

### Phase 1.3: OCC Conflict Detection

**Files to modify:**

- `src/raft/storage_sqlite.rs` - Add conflict validation

**Tasks:**

1. In `apply_optimistic_transaction()`, validate read set before applying writes:

   ```rust
   // Check all keys in read_set still have expected versions
   for (key, expected_version) in &read_set {
       let current: i64 = conn.query_row(
           "SELECT version FROM state_machine_kv WHERE key = ?",
           params![key],
           |row| row.get(0)
       ).unwrap_or(0);

       if current != *expected_version {
           return Ok(AppResponse::ConflictError {
               key: key.clone(),
               expected: *expected_version,
               actual: current,
           });
       }
   }
   // If no conflicts, apply write_set
   ```

2. Add `AppResponse::ConflictError` variant

3. Add tests for concurrent transaction conflicts

### Phase 1.4: Client Transaction Builder

**Files to create:**

- `src/client/transaction.rs` - Transaction builder API

**Tasks:**

1. Create fluent builder:

   ```rust
   pub struct TransactionBuilder {
       read_set: Vec<(String, i64)>,
       write_set: Vec<WriteOp>,
   }

   impl TransactionBuilder {
       pub fn new() -> Self { ... }
       pub fn read(mut self, key: &str, version: i64) -> Self { ... }
       pub fn write(mut self, key: &str, value: &str) -> Self { ... }
       pub fn delete(mut self, key: &str) -> Self { ... }
       pub fn build(self) -> WriteCommand { ... }
   }
   ```

---

## Track B: P1 - Horizontal Scaling (Sharding)

### Problem

Single monolithic Raft cluster. All data replicated to all nodes. Cannot scale beyond ~100MB practical data size.

### Phase 2.1: Shard Router Design

**Files to create:**

- `src/sharding/mod.rs`
- `src/sharding/router.rs`
- `src/sharding/consistent_hash.rs`

**Tasks:**

1. Define core types:

   ```rust
   pub type ShardId = u32;

   pub struct ShardRange {
       pub shard_id: ShardId,
       pub start_key: String,  // inclusive
       pub end_key: String,    // exclusive
   }

   pub struct ShardRouter {
       shard_map: Arc<RwLock<Vec<ShardRange>>>,
       local_shards: HashSet<ShardId>,
   }
   ```

2. Implement consistent hashing (Jump hash recommended):

   ```rust
   impl ShardRouter {
       pub fn get_shard_for_key(&self, key: &str) -> ShardId {
           // Jump consistent hash
           let hash = seahash::hash(key.as_bytes());
           jump_consistent_hash(hash, self.num_shards())
       }
   }
   ```

3. Add comprehensive unit tests for key distribution

### Phase 2.2: Per-Shard RaftNode

**Files to modify:**

- `src/raft/node.rs` - Add shard awareness
- `src/raft/storage_sqlite.rs` - Add shard_id column

**Files to create:**

- `src/sharding/sharded_node.rs`

**Tasks:**

1. Create `ShardedRaftNode` wrapper:

   ```rust
   pub struct ShardedRaftNode {
       shards: Arc<RwLock<HashMap<ShardId, Arc<RaftNode>>>>,
       router: Arc<ShardRouter>,
       node_id: NodeId,
   }

   #[async_trait]
   impl KeyValueStore for ShardedRaftNode {
       async fn write(&self, request: WriteRequest) -> Result<WriteResult> {
           let key = request.command.primary_key();
           let shard_id = self.router.get_shard_for_key(&key);
           let shard = self.get_or_create_shard(shard_id).await?;
           shard.write(request).await
       }

       async fn scan(&self, request: ScanRequest) -> Result<ScanResult> {
           // Parallel scan across all shards, merge results
           let futures = self.shards.read().await
               .values()
               .map(|shard| shard.scan(request.clone()))
               .collect::<Vec<_>>();

           let results = futures::future::join_all(futures).await;
           self.merge_scan_results(results, &request)
       }
   }
   ```

2. Modify SQLite schema:

   ```sql
   ALTER TABLE state_machine_kv ADD COLUMN shard_id INTEGER DEFAULT 0;
   CREATE INDEX idx_shard_key ON state_machine_kv(shard_id, key);
   ```

---

## Constraints

1. **Tiger Style**: Respect all resource bounds in `src/raft/constants.rs`
2. **No panics**: Use `?` operator and snafu for error handling
3. **No HTTP**: Use Iroh for all network communication
4. **Tests required**: Add tests for all new functionality
5. **Backward compatible**: Existing APIs must continue to work
6. **Documentation**: Add doc comments to all public APIs

## Testing Strategy

1. **Unit tests**: Test individual functions in isolation
2. **Integration tests**: Test multi-node behavior with `madsim`
3. **Property tests**: Use `proptest` for edge cases
4. **Chaos tests**: Inject failures during operations

## Build Commands

```bash
# Build
nix develop -c cargo build

# Test (quick profile for iteration)
nix develop -c cargo nextest run -P quick

# Test specific module
nix develop -c cargo nextest run -E 'test(/storage/)'
nix develop -c cargo nextest run -E 'test(/sharding/)'

# Full test suite
nix develop -c cargo nextest run

# Check results
cat target/nextest/default/junit.xml
```

## Definition of Done

### P0 Complete When

- [ ] All write paths update version/revision metadata
- [ ] OptimisticTransaction command implemented
- [ ] Conflict detection rejects stale reads
- [ ] Transaction builder API available
- [ ] Tests pass for concurrent transactions
- [ ] < 5% overhead vs current CAS operations

### P1 Complete When

- [ ] ShardRouter correctly maps keys to shards
- [ ] ShardedRaftNode routes operations to correct shard
- [ ] Parallel scan merges results correctly
- [ ] 2-3 shard integration tests pass
- [ ] Benchmark shows linear scaling

## Getting Started

1. Read the progress tracker: `cat .claude/progress-2025-12-22.md`
2. Read the gap analysis: `cat .claude/foundationdb-etcd-gap-analysis-2025-12-22.md`
3. Explore current implementation: `src/raft/storage_sqlite.rs`, `src/api/mod.rs`
4. Start with Phase 1.1 (version tracking) - it's the smallest, most contained change
5. Run tests frequently: `nix develop -c cargo nextest run -P quick`

Begin implementation. Update `.claude/progress-2025-12-22.md` as you complete tasks.
