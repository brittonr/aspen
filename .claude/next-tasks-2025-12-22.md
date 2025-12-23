# Next Tasks Analysis - Aspen Project

**Date:** 2025-12-22
**Analysis:** Ultra Mode parallel agent synthesis
**Branch:** v3 (36 commits ahead of origin/v3)

---

## Executive Summary

After comprehensive analysis across code quality, security, testing, architecture, and recent development momentum, the following 5 tasks represent the highest-impact next steps for the Aspen project.

---

## Top 5 Recommended Tasks

### Task 1: Implement Gossip Discovery Fixes (Ready to Execute)

**Priority:** HIGH
**Effort:** 2-3 hours
**Risk:** Low
**Files:** `src/cluster/gossip_discovery.rs`, `src/raft/constants.rs`, `tests/support/mock_iroh.rs`

**Problem:**

1. Gossip receiver task exits on any transient error (network blips become fatal)
2. Announcer broadcasts every 10s without sender-side rate limiting
3. ALPN constants duplicated between production and test code

**Solution:**

- Add exponential backoff with bounded retry (max 5 retries) using existing `calculate_backoff_duration`
- Implement adaptive announcement interval (10s-60s based on success/failure)
- Consolidate ALPN constants to single source of truth

**Plan exists:** `.claude/plan-gossip-fixes-2025-12-19.md` (fully detailed, ready for implementation)

**Why now:** This is low-risk, contained, and improves network resilience for production deployments.

---

### ~~Task 2: Replace std::sync::RwLock in Async Context~~ **RESOLVED: FALSE POSITIVE**

**Status:** CLOSED - No action required
**Analysis Date:** 2025-12-22

**Finding:**
This task was based on INCORRECT analysis. The file `src/client/overlay.rs` **already uses `tokio::sync::RwLock`** (line 22: `use tokio::sync::RwLock;`). All async methods correctly use `.await` with locks.

**Other Files Analyzed:**

- `src/client/cache.rs`: Uses `std::sync::RwLock` - **CORRECT** (sync methods only, no `.await`)
- `src/auth/verifier.rs`: Uses `std::sync::RwLock` - **CORRECT** (sync methods only, no `.await`)
- `src/bin/aspen-fuse/inode.rs`: Uses `std::sync::RwLock` - **CORRECT** (FUSE callbacks are sync)
- `src/raft/storage.rs`: Uses `std::sync::RwLock` - **CORRECT** (documented for sync/async bridge)

**Conclusion:**
Per Tokio's own guidance: "Use `std::sync::RwLock` when locks are held briefly and NOT across `.await` points." All usages in the Aspen codebase follow this correctly.

---

### Task 3: Add Coordination Primitive Integration Tests

**Priority:** HIGH
**Effort:** 4-6 hours
**Risk:** Low (adding tests, not changing production code)
**Files:** `tests/coordination_*.rs` (new test files)

**Problem:**
13 coordination modules (distributed locks, leader election, queues, barriers, etc.) have only RPC serialization tests. Missing:

- Lock contention scenarios
- Leader election with failures
- Queue FIFO ordering verification
- RWLock reader/writer coordination
- Barrier synchronization

**Current coverage:** ~15% for coordination primitives vs ~85% for Raft/storage.

**Solution:**
Add integration tests using the madsim deterministic simulation framework to verify correctness under various failure scenarios.

**Why now:** These are new primitives (added mid-December) that need validation before production use.

---

### Task 4: Fix Audience Parsing Security Issue in Client Handler

**Priority:** MEDIUM-HIGH
**Effort:** 1 hour
**Risk:** Low
**File:** `src/protocol_handlers/client.rs` (line 302)

**Problem:**

```rust
let presenter = iroh::PublicKey::from_str(client_id).ok();
```

When `PublicKey::from_str()` fails, `presenter` becomes `None`. A Key-audience token will then incorrectly accept any request because the audience check is skipped. This is a potential authorization bypass.

**Solution:**

1. Log parse failures explicitly
2. Handle malformed client_id at connection time, not per-request
3. Return authentication error if client_id cannot be parsed

**Why now:** This is a security issue in the newly-added capability-based authorization system.

---

### Task 5: Increase SQLite Connection Pool Size

**Priority:** MEDIUM
**Effort:** 30 minutes
**Risk:** Low
**Files:** `src/raft/storage_sqlite.rs`, `src/raft/constants.rs`

**Problem:**
SQLite read pool is sized at 10 connections while `MAX_CONCURRENT_OPS = 1000`. Under load, this creates a bottleneck where 990 operations are blocked waiting for database connections.

**Solution:**

- Increase `DEFAULT_READ_POOL_SIZE` from 10 to 50-100
- Make pool size configurable via cluster config
- Add metrics for pool utilization

**Why now:** Simple change with direct performance impact for production deployments.

---

## Alternative Tasks (Next Sprint)

### Task 6: Extract Large Functions (Tiger Style Compliance)

**Files affected:**

- `handle_authenticated_raft_stream()`: 132 lines (limit: 70)
- `handle_client_request()`: 95 lines (limit: 70)

### Task 7: Complete Delegation Chain Verification

**File:** `src/auth/builder.rs` (lines 174-183)
**Issue:** MVP delegation depth tracking returns 1 or 2, not actual chain depth

### Task 8: Address proc-macro-error Dependency

**Issue:** `proc-macro-error v0.4.12` is unmaintained (RUSTSEC-2024-0370)
**Path:** `iroh-blobs -> bao-tree -> genawaiter -> proc-macro-error`
**Action:** Coordinate with iroh maintainers or evaluate if iroh-blobs update available

### Task 9: Split protocol_handlers.rs

**File:** 6,094 lines - should be modular directory structure
**Note:** Recent commit `12359cc8` started this work

### Task 10: Add Unit Tests for API Traits

**Files:** 13 modules with TODO comments indicating missing unit test coverage
**Impact:** Core traits (`ClusterController`, `KeyValueStore`) lack isolated validation

---

## Decision Matrix

| Task | Impact | Effort | Risk | Readiness |
|------|--------|--------|------|-----------|
| 1. Gossip Fixes | High | Medium | Low | Ready (plan exists) |
| ~~2. RwLock Fix~~ | ~~High~~ | ~~Low~~ | ~~Low~~ | **FALSE POSITIVE** |
| 3. Coordination Tests | High | Medium | Low | Ready |
| 4. Auth Security | High | Low | Low | Ready |
| 5. SQLite Pool | Medium | Low | Low | Ready |

---

## Recommended Execution Order

*Updated 2025-12-22: Task 2 removed (false positive)*

1. **Task 4** (Auth security) - 1 hour, security issue
2. **Task 5** (SQLite pool) - 30 min, easy win
3. **Task 1** (Gossip fixes) - 2-3 hours, plan ready
4. **Task 3** (Coordination tests) - 4-6 hours, validates new features

**Total estimated effort:** 7.5-10.5 hours

---

## Monitoring & Rollback

All recommended tasks are:

- Contained to specific files
- Testable in isolation
- Reversible via git revert
- Verifiable with existing test suite (`cargo nextest run`)

---

*Analysis conducted using parallel subagents: codebase exploration, test coverage analysis, recent commits review, security analysis, and architecture completeness review.*
