# Tiger Style Code Quality Audit

**Date**: 2025-12-30
**Codebase**: Aspen (Blixard)
**Overall Compliance**: 8.5/10

---

## Executive Summary

| Category | Status | Score |
|----------|--------|-------|
| Unwrap/Expect | Excellent | 10/10 |
| Panic/Todo/Unimplemented | Excellent | 10/10 |
| Lock-Across-Await | Exemplary | 10/10 |
| Unsafe Blocks | Excellent | 10/10 |
| Function Length | Needs Work | 6/10 |
| Unnecessary Clone | Minor Issues | 7/10 |
| Unbounded Operations | 1 Critical | 7/10 |
| Error Handling | Good | 8/10 |

---

## 1. Unwrap/Expect Usage

**Status**: PASS - No action needed

**Findings**: Zero HIGH or MEDIUM severity issues. All `.unwrap()` uses have fallbacks (`.unwrap_or()`, `.unwrap_or_default()`) or are provably safe with documentation.

**Exemplary Pattern** (`src/raft/auth.rs:254`):
```rust
.expect("HMAC accepts any key size per RFC 2104")
```
Well-documented exception with mathematical proof of safety.

**Common Safe Patterns Found**:
- `unwrap_or(0)` for metrics collection
- `unwrap_or_default()` for time operations
- `unwrap_or(DEFAULT_SCAN_LIMIT)` for request limits

---

## 2. Panic/Todo/Unimplemented

**Status**: PASS - No action needed

**Findings**:
- 42 occurrences in test code (appropriate)
- 1 production `unreachable!()` in `src/raft/node.rs:675` (justified - exhaustive match)

**Justified Production Use**:
```rust
// src/raft/node.rs:675
_ => unreachable!("cas_succeeded only set for CAS operations")
```
This follows an exhaustive pattern match where the flag is only set for specific operations.

---

## 3. Lock-Across-Await Safety

**Status**: EXEMPLARY - No action needed

**Findings**: No locks held across await points. The codebase demonstrates excellent async safety practices.

**Best Practice Example** (`src/raft/connection_pool.rs:484-534`):
```rust
// Phase 1: Collect under read lock (no awaits)
let candidates: Vec<_> = {
    let connections = pool.read().await;
    connections.iter().map(|...| ...).collect()
};  // Lock released

// Phase 2: Process WITHOUT lock (awaits allowed)
for (node_id, conn) in candidates {
    let is_idle = conn.is_idle(timeout).await;  // No lock held
}

// Phase 3: Write lock only for quick removal
let mut connections = pool.write().await;
for node_id in to_remove {
    connections.remove(&node_id);
}
```

---

## 4. Unsafe Blocks

**Status**: PASS - No action needed

**Findings**: All 6 `unsafe` blocks are necessary FFI/syscall patterns with proper safety documentation.

| Location | Type | Documented |
|----------|------|------------|
| `src/utils.rs:94` | C struct init (statvfs) | Yes |
| `src/utils.rs:97` | FFI syscall (statvfs) | Yes |
| `src/bin/aspen-fuse/fs.rs:117` | C struct init (stat64) | Yes |
| `src/bin/aspen-fuse/fs.rs:880` | C struct init (statvfs64) | Yes |
| `src/bin/aspen-fuse/main.rs:146` | FFI syscall (getuid) | Yes |
| `src/bin/aspen-fuse/main.rs:147` | FFI syscall (getgid) | Yes |

---

## 5. Function Length

**Status**: NEEDS WORK - 155 functions exceed 70-line limit

### Critical (>200 lines) - Refactor Priority

| File | Function | Lines | Issue |
|------|----------|-------|-------|
| `src/cluster/bootstrap.rs` | `parse_peer_addresses` | 1,001 | Monolithic parser |
| `src/protocol_handlers/handlers/cluster.rs` | `handle_add_learner` | 437 | Mixed validation + Raft |
| `src/client_rpc.rs` | `to_operation` | 423 | Exhaustive match |
| `src/cluster/gossip_discovery.rs` | `spawn` | 385 | Multiple task spawns |
| `src/bin/aspen-node.rs` | `main` | 384 | Config + error display |
| `src/protocol_handlers/log_subscriber.rs` | `handle_log_subscriber_connection` | 327 | Connection lifecycle |
| `src/docs/store.rs` | `spawn_sync_event_listener` | 312 | Event loop |
| `src/bin/aspen-cli/commands/git.rs` | `git_clone` | 284 | Sequential operations |

### High (150-200 lines) - Refactor Soon

| File | Function | Lines |
|------|----------|-------|
| `src/protocol_handlers/handlers/coordination.rs` | `handle` | 280 |
| `src/bin/aspen-tui/app.rs` | `on_key` | 279 |
| `src/protocol_handlers/handlers/forge.rs` | `handle` | 263 |
| `src/raft/node.rs` | `write` | 258 |

### Recommended Refactoring Pattern

**Before** (385-line `spawn()`):
```rust
pub async fn spawn(...) -> Result<Self> {
    // 40 lines of setup
    let announcer_task = tokio::spawn(async move {
        // 120 lines of announcer logic
    });
    let receiver_task = tokio::spawn(async move {
        // 140 lines of receiver logic
    });
    // 20 lines of wrap-up
    Ok(Self { ... })
}
```

**After** (3 focused functions):
```rust
pub async fn spawn(...) -> Result<Self> {
    let (announcer, cancel_a) = Self::spawn_announcer(...).await?;
    let (receiver, cancel_r) = Self::spawn_receiver(...).await?;
    Ok(Self { announcer, receiver, ... })
}

async fn spawn_announcer(...) -> Result<(JoinHandle, CancellationToken)> { /* 60 lines */ }
async fn spawn_receiver(...) -> Result<(JoinHandle, CancellationToken)> { /* 65 lines */ }
```

### Justified Long Functions

Some functions are acceptable despite length:
- `src/raft/storage.rs::apply` (235 lines) - Large match with many operation types, each arm is 2-4 lines
- State machine transitions where seeing complete flow aids understanding

---

## 6. Unnecessary Clone Calls

**Status**: MINOR ISSUES

### High Priority

| Location | Pattern | Suggested Fix |
|----------|---------|---------------|
| `src/raft/node.rs:235,255,408,415,428,440` | `metrics.borrow().clone()` | Dereference without clone for reads |
| `src/raft/write_batcher.rs:211,228` | `key.clone(), value.clone()` in function args | Change signatures to accept `&str` |
| `src/raft/node.rs:524-546,597,606-609,625-626,670,673` | Batch operation clones | Clone at `AppRequest` creation, not in map |
| `src/node/mod.rs:472` | `Arc::new(endpoint.clone())` | Double allocation - rethink Arc strategy |

### Medium Priority

| Location | Pattern | Suggested Fix |
|----------|---------|---------------|
| `src/bin/aspen-cli/output.rs:185,200,222,241,284,309` | `String::from_utf8(v.clone())` | Clone only in error path |
| `src/dns/authority.rs:206` | TXT strings clone | Pass directly to `TXT::new()` |
| `src/raft/write_batcher.rs:618,628,636,684` | Batch conversion clones | Consolidate cloning |

### Recommended Fix Pattern

**Before**:
```rust
async fn batch_set(&self, key: String, value: String) { ... }

// Called with:
self.batch_set(key.clone(), value.clone()).await
```

**After**:
```rust
async fn batch_set(&self, key: &str, value: &str) {
    // Clone internally only when inserting into pending batch
    self.pending.insert(key.to_owned(), value.to_owned());
}

// Called with:
self.batch_set(&key, &value).await
```

---

## 7. Unbounded Operations

**Status**: 1 CRITICAL ISSUE

### Critical

**`src/api/inmemory.rs:691-703`** - `DeterministicKeyValueStore.scan()`

```rust
let mut matching: Vec<_> = inner
    .iter()
    .filter(|(k, _)| k.starts_with(&request.prefix))
    .filter(|(k, _)| { /* continuation check */ })
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect();  // UNBOUNDED: Collects ALL matching keys before slicing
```

**Risk**: With millions of keys matching a prefix, this allocates unlimited memory before applying the limit.

**Fix**: Use `.take(limit)` before `.collect()`:
```rust
let matching: Vec<_> = inner
    .iter()
    .filter(...)
    .take(limit)  // Stop early
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect();
```

### Moderate (Count operations iterate entire tables)

| Location | Function | Issue |
|----------|----------|-------|
| `src/raft/storage_shared.rs:675-693` | `count_expired_keys()` | No batch limit |
| `src/raft/storage_shared.rs:698-716` | `count_keys_with_ttl()` | No batch limit |
| `src/raft/storage_shared.rs:882-898` | `count_expired_leases()` | No batch limit |
| `src/raft/storage_shared.rs:903-919` | `count_active_leases()` | No batch limit |

**Risk**: In databases with millions of entries, causes unbounded read transaction duration and potential CPU spikes during metrics collection.

**Fix**: Add optional `max_count` parameter or use sampling for metrics.

### Minor

**`src/raft/storage_shared.rs:558`** - Uses `MAX_BATCH_SIZE` (1,000) instead of `MAX_SCAN_RESULTS` (10,000) for scan operations.

---

## 8. Error Handling

**Status**: GOOD (8/10)

### Issue 1: Anyhow in Library Code

Tiger Style prefers structured `snafu` errors in library code. These files use `anyhow`:

| File | Lines |
|------|-------|
| `src/protocol_handlers/raft.rs` | 100, 153, 170-171, 180-181 |
| `src/protocol_handlers/raft_authenticated.rs` | 290, 299, 284-285, 294-295 |
| `src/protocol_handlers/log_subscriber.rs` | 267, 271 |
| `src/sharding/automation.rs` | 139, 168, 180, 232, 269 |
| `src/testing/madsim_tester.rs` | 1795, 1812 |

**Fix**: Create structured error types:
```rust
// Instead of:
return Err(anyhow::anyhow!("vote failed: {:?}", err));

// Use:
#[derive(Debug, Snafu)]
pub enum RaftProtocolError {
    #[snafu(display("vote RPC failed: {source}"))]
    VoteFailed { source: RaftError },
}
```

### Issue 2: Log-and-Return Pattern (~25 instances)

```rust
// src/protocol_handlers/raft.rs:170-171
error!(error = ?err, "vote RPC failed with fatal error");
return Err(anyhow::anyhow!("vote failed: {:?}", err));  // Duplicate!
```

**Fix**: Log at one layer only (usually the caller) or use structured errors that carry context.

### Issue 3: Generic Error Type Names

Tiger Style recommends `VerbObjectError` pattern:

| Current | Suggested |
|---------|-----------|
| `TupleError` | `TupleDecodeError` |
| `SubspaceError` | `SubspacePrefixError` |
| `SqlError` | `SqlExecutionError` |
| `OverlayError` | `OverlayMountError` |

### Strengths

- Excellent `snafu` usage (23 files with proper error enums)
- Clear error variants with context fields
- Proper `From` implementations for error conversion chains
- 708 uses of `.context()` for error enrichment

---

## Action Items

### Priority 1: Critical Fixes
- [ ] Fix unbounded scan in `src/api/inmemory.rs` - stop collecting at limit

### Priority 2: High Impact Refactoring
- [ ] Refactor `parse_peer_addresses` (1,001 lines) into smaller parsers
- [ ] Change `write_batcher` signatures to accept `&str` instead of `String`
- [ ] Add batch limits to `count_expired_keys`/`count_keys_with_ttl` operations

### Priority 3: Code Quality
- [ ] Replace `anyhow` with structured `snafu` errors in protocol handlers
- [ ] Remove `metrics.borrow().clone()` pattern - dereference without clone
- [ ] Extract task spawning from `gossip_discovery.rs::spawn()` function
- [ ] Rename generic error types to `VerbObjectError` pattern

### Priority 4: Nice to Have
- [ ] Refactor remaining functions >150 lines
- [ ] Add context to `.ok()?` chains in gossip discovery
- [ ] Review Arc endpoint wrapping strategy in `node/mod.rs`

---

## Appendix: Tiger Style Quick Reference

### Safety Principles
- Avoid `.unwrap()` and `.expect()` - use `?`, `.context()`, or pattern matching
- Avoid `panic!()`, `todo!()`, `unimplemented!()` in production
- Avoid `unsafe` unless necessary; document safety invariants
- Set fixed limits on loops, queues, and data structures

### Async/Concurrency Safety
- Never hold locks across `.await` points
- Prefer `tokio::sync` over `std::sync` in async code
- Use structured concurrency (`JoinSet`, `TaskTracker`)

### Performance Principles
- Avoid unnecessary `.clone()` - prefer borrowing
- Batch operations to amortize costs
- Keep functions under 70 lines

### Error Handling
- Add context to errors with `.context()` or `.whatever_context()`
- Don't log and return the same error
- Use `VerbObjectError` naming pattern
