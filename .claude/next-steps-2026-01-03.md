# Aspen Development Next Steps

Created: 2026-01-03T20:34:58Z
Updated: 2026-01-04T03:30:00Z (ULTRA Analysis v3)

## Current State Summary

- **Lines of Code**: ~27,440 total (21,000 production + 6,400 tests)
- **Test Results**: 545 tests passing, 1 failing (`test_failure_cascade` timeout), quick profile
- **Build Status**: Compiles successfully (warnings remain - see breakdown below)
- **Recent Focus**: Job queue system, worker pools, VM execution, scheduling, affinity
- **Branch**: `v3` with 244 modified files

---

## ULTRA Analysis Synthesis (v3)

This update reflects verification of previous fixes and current priorities.

**Previous Session Fixes (VERIFIED COMPLETE)**:

- `unimplemented!()` calls in production code - RESOLVED
- `partial_cmp().unwrap()` on floats - RESOLVED
- Compilation errors in `worker_service.rs` - RESOLVED

---

## TIER 0: REMAINING CRITICAL ISSUES

### 1. Test Failure: `test_failure_cascade`

**Status**: ACTIVE FAILURE

- **File**: `tests/madsim_job_worker_test.rs`
- **Issue**: 30-second timeout with seed 909
- **Reproduce**: `MADSIM_TEST_SEED=909 cargo nextest run test_failure_cascade`
- **Impact**: Job worker failover cascade not validating

**Investigation Needed**:

- Check if timeout is too aggressive for complex failure scenario
- Verify failover logic completes within expected bounds
- Consider if test expectations match implementation

### 2. Remaining `.unwrap()` Calls on SystemTime (TIGER STYLE)

Several files still use `.unwrap()` on `SystemTime::now()` operations:

| File | Line | Current | Fix |
|------|------|---------|-----|
| `progress.rs` | 105, 150, 185 | `.unwrap()` | `.unwrap_or_default()` |
| `cob/store.rs` | 157, 264 | `.unwrap()` | `.unwrap_or_default()` |
| `replay.rs` | 363 | `.unwrap()` | `.unwrap_or(0)` |

---

## TIER 1: HIGH PRIORITY (This Week)

### 4. Security Findings

**HIGH Severity (2 items)**:

| Issue | Location | Risk | Fix |
|-------|----------|------|-----|
| System time unwrap | `aspen-transport/src/rpc.rs:54` | Panic on clock error | Use `.unwrap_or_default()` |
| HMAC placeholder | `aspen-transport/src/rpc.rs:58-62` | Weak auth verification | Document or implement proper HMAC |

**Security Strengths Confirmed**:

- No unsafe code blocks
- Excellent Tiger Style resource bounds
- Strong error sanitization (no info leakage)
- Ed25519 + BLAKE3 authentication
- Comprehensive rate limiting

### 5. Git CAS Push Implementation

**File**: `aspen-cli/src/bin/aspen-cli/commands/git.rs:834`
**Issue**: `expected: None` always passed - race condition in concurrent pushes
**Impact**: Data loss risk

**Fix**: Fetch current ref value before CAS operation:

```rust
let current_ref = client.send(ClientRpcRequest::ForgeGetRef { ... }).await?;
let expected = current_ref.hash;
```

### 6. Snapshot Integrity Hash Missing

**File**: `aspen-raft/src/storage_shared.rs:2164`
**Issue**: `integrity: None` - no corruption detection
**Impact**: Silent data corruption after restart

**Fix**: Calculate and verify SHA256 hash of snapshot data

---

## TIER 2: MEDIUM PRIORITY (This Sprint)

### 7. Test Coverage Critical Gaps

| Module | LOC | Tests | Gap |
|--------|-----|-------|-----|
| `aspen-jobs/scheduler.rs` | 380 | 0 | Schedule execution core feature |
| `aspen-jobs/affinity.rs` | 500 | 0 | Data locality (added Dec 2025) |
| `aspen-jobs/parse.rs` | 300 | 0 | Schedule format parsing |
| `aspen-coordination/barrier.rs` | 250 | 0 | Distributed barriers |
| `aspen-coordination/queue.rs` | 600 | 0 | Distributed FIFO queue |
| `aspen-forge/git/bridge/importer.rs` | 300 | 0 | Git-to-Forge conversion |

**Estimated Tests Needed**: 220-310 additional tests

### 8. ALPN Constant Consolidation

**Issue**: `CLIENT_ALPN` defined in 3 places

- `aspen-transport/src/constants.rs:26` (PRIMARY)
- `aspen-rpc-handlers/src/client.rs:46` (DUPLICATE - REMOVE)
- `aspen-client/src/constants.rs:4` (re-export - OK)

### 9. Warning Cleanup (314 total)

| Category | Count | Priority | Action |
|----------|-------|----------|--------|
| Unused RPC types | 130 | Medium | Add `#![allow(dead_code)]` with doc |
| Unused imports | 55 | High | Remove each import |
| Unused variables | 8 | High | Prefix with `_` |
| Missing docs | 70 | Low | Add field docs |
| Other | 51 | Low | Various fixes |

**Quick Wins** (1-2 hours):

- Remove 55 unused imports
- Prefix 8 variables with `_`

---

## TIER 3: ARCHITECTURAL IMPROVEMENTS

### 10. Function Size Violations (Tiger Style)

| File | Function | Lines | Limit |
|------|----------|-------|-------|
| `bootstrap.rs` | `bootstrap_node()` | 151 | 70 |
| `bootstrap.rs` | `bootstrap_sharded_node()` | 241 | 70 |
| `bootstrap.rs` | `wire_docs_sync_services()` | 81 | 70 |
| `network.rs` | `send_rpc()` | 192 | 70 |

**Recommendation**: Extract logical sub-functions

### 11. Module Coupling

**`aspen-cluster/src/bootstrap.rs`**:

- 46 use statements
- 34 internal crate dependencies
- Direct instantiation of 8+ concrete types

**Recommendation**: Introduce `StorageFactory` and `DiscoveryFactory` traits for cleaner dependency injection

---

## TODO Comments Inventory (26 Total)

### Blocking (5)

| Location | Issue | Impact |
|----------|-------|--------|
| `git.rs:834` | CAS push missing expected value | Race condition |
| `storage_shared.rs:2164` | Snapshot integrity hash | Data corruption |
| `storage.rs:1625` | In-memory transaction support | Test failures |
| `connection_pool.rs:406` | Auth ALPN configuration | Security bypass |
| `forge.rs:2089` | Federation listing stub | Empty results |

### Medium (7)

- Auth format unification (client.rs)
- Tag signing key flag (tag.rs)
- Blob deletion (blob.rs)
- Gossip topology sync (discovery.rs)
- Watch status query (watch.rs)
- Span aggregation (observability.rs)
- Cluster ticket duplication (cluster.rs)

### Low (14)

- Code organization, test coverage, documentation

---

## Recommended Action Sequence

### Phase 1: Critical Path (Today) - UPDATED

**Completed in Previous Session**:

- [x] Fix `unimplemented!()` in `RaftNode::kv_store()` and `cluster_controller()`
- [x] Fix `clone_for_task()` in `DistributedWorkerCoordinator`
- [x] Fix `partial_cmp().unwrap()` calls

**Remaining Critical**:

1. Investigate `test_failure_cascade` timeout (seed 909)
2. Fix remaining SystemTime `.unwrap()` calls in `progress.rs`, `cob/store.rs`, `replay.rs`

### Phase 2: High Value (This Week)

3. Implement Git CAS push expected value retrieval
4. Add snapshot integrity hash
5. Remove 55+ unused imports
6. Prefix unused variables with `_`
7. Remove ALPN constant duplication

### Phase 3: Test Coverage (Next Sprint)

8. Add `scheduler.rs` tests (470 LOC, 0 tests) - 20-30 tests
9. Add `affinity.rs` tests (553 LOC, 0 tests) - 15-20 tests
10. Add `parse.rs` tests (805 LOC, 0 tests) - 20-25 tests
11. Add coordination primitive tests (barrier, queue) - 80+ tests
12. Add forge/git/bridge tests - 30+ tests

### Phase 4: Cleanup (Backlog)

13. Function extraction for Tiger Style compliance
14. Documentation for unused RPC types
15. Module coupling improvements in bootstrap.rs

---

## Files Modified (Session)

- `crates/aspen-cluster/src/worker_service.rs` - Added `local_shards` field to WorkerMetadata

---

## Metrics Summary

| Metric | Count | Status |
|--------|-------|--------|
| Production `panic!()` | 0 | OK |
| Production `todo!()` | 0 | OK |
| Production `unimplemented!()` | 0 | RESOLVED |
| Production `unsafe` | 0 | OK |
| Warnings | ~120 | Reduced from 314 |
| Tests (quick profile) | 545 | 1 failing |
| Critical TODOs | 5 | Needs attention |

---

## Warning Breakdown (Current)

| Category | Count | Priority |
|----------|-------|----------|
| Missing struct field docs | 70 | Low |
| Unused imports | ~20 | High |
| Unused variables | ~8 | High |
| Dead code (RPC types) | ~15 | Medium |
| Other | ~7 | Low |

---

## Security Audit Summary

| Area | Status | Notes |
|------|--------|-------|
| Unsafe code | CLEAN | None found |
| Input validation | STRONG | Prefix protection, SQL whitelist |
| Resource limits | EXCELLENT | Tiger Style bounds enforced |
| Auth/AuthZ | STRONG | Ed25519 + BLAKE3 capability tokens |
| Error sanitization | EXCELLENT | No info leakage |
| Rate limiting | GOOD | Gossip limits, configurable client limits |
| Crypto | STRONG | Standard algorithms, no custom crypto |

---

## Next Session Checklist

- [x] Fix 3 `unimplemented!()` calls - DONE
- [x] Fix 2 `partial_cmp().unwrap()` calls - DONE
- [ ] Investigate `test_failure_cascade` timeout (seed 909)
- [ ] Fix SystemTime `.unwrap()` in `progress.rs`, `cob/store.rs`, `replay.rs`
- [ ] Remove unused imports
- [ ] Add tests for untested modules (scheduler, affinity, parse)
- [ ] Run full test suite to verify
