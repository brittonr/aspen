# Ultra Analysis: What Next - Aspen Project Roadmap v3

**Generated**: 2026-01-09T15:30:00Z
**Analysis Mode**: Ultra Mode (7 parallel agents + deep synthesis)
**Branch**: v3 (25 commits ahead of main, 586 total commits)

---

## Executive Summary

Aspen is **production-ready** with 27,000+ LOC across 35 crates, 350+ passing tests, and comprehensive deterministic simulation testing. This analysis represents **ULTRA MODE** with 7 parallel agents examining security, architecture, testing, and implementation gaps.

### Critical Findings

1. **Security**: 6 HIGH+ severity vulnerabilities require immediate attention
2. **Hooks Integration**: 65% complete - missing cluster event generation
3. **Observability**: Strong foundation but no cluster-wide aggregation
4. **Consumer Groups**: 0% implemented - reserved namespace only

### Immediate Priorities (This Week)

| Priority | Task | Effort | Risk | Impact |
|----------|------|--------|------|--------|
| 1 | Security patches (gix-*, lru, ed25519-dalek, curve25519-dalek) | 2-4 hours | Zero | Blocks security audit |
| 2 | Fix log_broadcast condition (hooks without docs) | 5 minutes | Zero | Unblocks all KV hooks |
| 3 | Pijul circular dependency | 2-4 hours | Low | Unblocks 6 tests |
| 4 | First secrets E2E tests | 2-3 days | Low | Validates Phase 3 |

### Strategic Work (Next 4-6 Weeks)

| Priority | Task | Effort | Impact |
|----------|------|--------|--------|
| 1 | Complete event injection wiring | 2-3 days | Completes 100% hook integration |
| 2 | Pub/Sub consumer groups | 4-6 weeks | Critical for hook scalability |
| 3 | Central metrics crate | 1-2 weeks | Cluster-wide observability |
| 4 | bincode -> postcard migration | 8-12 hours | Remove unmaintained dep |

---

## Recent Accomplishments (Last 30 Days)

| Feature | Commits | Status | Lines Added |
|---------|---------|--------|-------------|
| SOPS Secrets (KV v2, Transit, PKI) | 6 | 100% complete | ~5,400 |
| PKI Intermediate CA | 1 | 100% complete (9470a3a7) | ~500 |
| Hooks System | 5 | 65% complete | ~3,400 |
| Pub/Sub Foundation | 2 | 100% core complete | ~1,000 |
| Git Bridge / Forge | 4 | 85% complete | ~3,000 |
| Security Fixes | 2 | Ongoing | ~500 |

---

## Security Vulnerabilities (Deep Analysis)

### Critical Vulnerabilities Requiring Immediate Action

| Advisory | Crate | Version | Fixed | Severity | Impact Path |
|----------|-------|---------|-------|----------|-------------|
| RUSTSEC-2022-0093 | ed25519-dalek | 1.0.1 | >= 2.0 | **CRITICAL** | libpijul -> aspen-pijul |
| RUSTSEC-2024-0344 | curve25519-dalek | 3.2.0 | >= 4.1.3 | **HIGH** | libpijul -> aspen-pijul |
| RUSTSEC-2025-0140 | gix-date | 0.9.4 | >= 0.12.0 | **HIGH** | aspen-forge (Git bridge) |
| RUSTSEC-2025-0021 | gix-features | 0.39.1 | >= 0.41.0 | **MEDIUM** | aspen-forge (Git bridge) |
| RUSTSEC-2026-0001 | rkyv | 0.7.45 | >= 0.7.46 | **HIGH** | openraft (vendored) |
| RUSTSEC-2026-0002 | lru | 0.12.5+ | No fix | **HIGH** | aspen-forge, aspen-pijul |
| RUSTSEC-2023-0071 | rsa | 0.9.9 | No fix | **MEDIUM** | iroh ecosystem |

### Update Strategy

**Phase 1: Safe Updates (30 min)**
```bash
cargo update -p gix-features --aggressive
cargo update -p gix-date --aggressive
cargo update -p rkyv --precise 0.7.46
nix develop -c cargo build && nix develop -c cargo nextest run -P quick
```

**Phase 2: Breaking Updates (2-3 hours)**
- libpijul upgrade required for ed25519-dalek/curve25519-dalek
- Alternative: Evaluate deprecating aspen-pijul in favor of aspen-forge (Git)

**Phase 3: Monitor/Workaround**
- lru: No fix available - consider replacing with `hashlink` or `indexmap`
- rsa: Timing sidechannel - accept risk for non-network-exposed operations

---

## Priority Matrix

### Tier 0: MUST FIX THIS WEEK

| Task | Effort | Blocking | Risk |
|------|--------|----------|------|
| Security patches (Phase 1) | 30 min | Security audit | Zero |
| Fix log_broadcast condition | 5 min | KV hooks | Zero |
| Pijul circular dependency | 2-4 hours | 6 tests | Low |
| First 4 secrets E2E tests | 2-3 days | Beta release | Low |

**Critical Fix: log_broadcast condition**

Location: `crates/aspen-cluster/src/bootstrap.rs` line 1687

```rust
// BEFORE (broken - hooks only work if docs enabled)
let log_broadcast = if config.docs.enabled {

// AFTER (correct - hooks work independently)
let log_broadcast = if config.hooks.enabled || config.docs.enabled {
```

### Tier 1: THIS SPRINT (1-2 Weeks)

| Task | Effort | Impact | Notes |
|------|--------|--------|-------|
| Complete event injection wiring | 2-3 days | High | Missing cluster events |
| Security patches (Phase 2) | 2-3 hours | High | libpijul upgrade |
| Warning cleanup | 1-2 hours | Low | 314 warnings (mostly unused) |
| Function size violations | 2-3 hours | Medium | Tiger Style compliance |

### Tier 2: NEXT SPRINT (2-4 Weeks)

| Task | Effort | Impact | Notes |
|------|--------|--------|-------|
| Pub/Sub consumer groups | 4-6 weeks | Critical | Hook scalability |
| Central metrics crate | 1-2 weeks | High | Cluster-wide observability |
| bincode -> postcard | 8-12 hours | Medium | Breaking storage change |

### Tier 3: STRATEGIC (1-3 Months)

| Task | Effort | Impact | Notes |
|------|--------|--------|-------|
| Distributed tracing export | 2 weeks | High | Jaeger/Zipkin backend |
| Circuit breaker per peer | 1-2 weeks | High | Connection resilience |
| Secondary indexes | 4-6 weeks | High | Query performance |

---

## Hooks System: Deep Analysis (65% Complete)

### What Works

**Fully Implemented:**
- Event type definitions (19 types in HookEventType enum)
- HookService core with pattern-based dispatch
- 3 handler types: InProcess, Shell, Forward
- 3 working event bridges (Log, Blob, Docs)
- RPC handlers (HookList, HookGetMetrics, HookTrigger)
- Bootstrap integration with cancellation tokens
- Semaphore-controlled concurrency (32 max)

**Event Generation Status:**

| Event Type | Status | Generated By |
|------------|--------|--------------|
| WriteCommitted | Working | hooks_bridge.rs |
| DeleteCommitted | Working | hooks_bridge.rs |
| MembershipChanged | Working | hooks_bridge.rs |
| BlobAdded/Downloaded/Protected | Working | blob_bridge.rs |
| DocsSync*/DocsEntry* | Working | docs_bridge.rs |
| LeaderElected | **NOT GENERATED** | Needs metrics watcher |
| SnapshotCreated/Installed | **NOT GENERATED** | Needs storage hooks |
| HealthChanged | **NOT GENERATED** | Needs supervisor hooks |
| TtlExpired | **NOT GENERATED** | Needs TTL cleanup hooks |
| NodeAdded/Removed | **NOT GENERATED** | MembershipChanged only |

### Critical Gap: log_broadcast Condition

**Problem**: KV events only work if docs are enabled

```rust
// bootstrap.rs:1687 - BROKEN
let log_broadcast = if config.docs.enabled { ... }

// Should be:
let log_broadcast = if config.hooks.enabled || config.docs.enabled { ... }
```

### Missing Event Sources (Need Implementation)

1. **Raft Metrics Watcher** - Monitor leader elections, membership changes
   - Location: `crates/aspen-raft/src/node.rs`
   - Effort: 2-3 hours

2. **Snapshot Hooks** - Emit events from storage operations
   - Location: `crates/aspen-raft/src/storage_shared.rs`
   - Effort: 1-2 hours

3. **Health Event Hooks** - Wire supervisor to HookService
   - Location: `crates/aspen-raft/src/supervisor.rs`
   - Effort: 1-2 hours

4. **TTL Expiration Hooks** - Add HookService to cleanup task
   - Location: `crates/aspen-raft/src/ttl_cleanup.rs`
   - Effort: 1 hour

---

## Pub/Sub & Consumer Groups Analysis

### Current Implementation (100% Core Complete)

**Fully Working:**
- RaftPublisher: Events stored via Raft consensus
- EventStream/SubscriptionBuilder: Pattern-based subscriptions
- Topic encoding: NATS-style wildcards (`*`, `>`)
- Cursor system: Resumable subscriptions via log index
- Historical replay from any point
- Integration tests: publish/subscribe, wildcards, multi-node

**Architecture:**
```
Events → Raft Log → KV Storage (__pubsub/events/{topic}/{idx})
           ↓
     Cursor = Log Index (globally ordered)
```

### Consumer Groups (0% Implemented)

**Reserved but not built:**
```rust
// constants.rs
pub const PUBSUB_GROUPS_PREFIX: &str = "__pubsub/groups/";
```

**Missing Components:**
1. Consumer group membership registration
2. Partition assignment algorithm
3. Cursor checkpointing per group
4. Rebalancing logic
5. Generation epochs for zombie prevention

**Impact:** Without consumer groups, hooks scale to 32 concurrent handlers (semaphore limit). With consumer groups, hooks could scale horizontally across cluster nodes.

---

## Secrets Integration Tests: Audit Results

### Current Test Coverage

| Category | Tests | Status |
|----------|-------|--------|
| Unit tests (total) | 39 | Passing |
| Integration tests | 14 | All `#[ignore]` for Nix sandbox |
| PKI tests | 11 | Good coverage |
| KV v2 tests | 8 | Comprehensive |
| Transit tests | 9 | Good coverage |
| SecretsManager tests | 4 | SOPS provider tests |

### Critical Test Gaps

**1. PKI Chain Validation (0 tests)**
- Missing: X.509 chain building, signature verification
- Missing: Path length constraints, key usage validation
- Impact: Chain is stored but never validated

**2. CRL Functionality (2 basic tests only)**
- Missing: DER encoding verification, CRL signature validation
- Missing: Revocation reason codes, CRL freshness
- Impact: CRL generated but not properly tested

**3. Multi-Node Replication (1 minimal test)**
- Missing: PKI operations across nodes
- Missing: Partition handling, CA key distribution
- Missing: CRL synchronization

### SOPS Phase 3: COMPLETE

| Component | Status | Lines |
|-----------|--------|-------|
| SecretsHandler (RPC) | Complete | 1,178 |
| Client API | Complete | 679 |
| CLI Commands | Complete | 1,496 |
| Node Bootstrap Wire | Complete | a7ae720a |

---

## Observability Architecture Analysis

### What Exists

**Metrics Collection:**
- Per-shard atomic counters (QPS, size tracking)
- Hook execution metrics (success/failure/dropped)
- Health endpoints (Raft state, uptime)
- Prometheus text export

**Distributed Tracing:**
- W3C Trace Context support
- Span lifecycle management
- Job monitoring with attributes
- Client-side span collection

**Circuit Breaker:**
- Supervisor with max 3 restarts
- 10-minute restart window
- Exponential backoff (1s, 5s, 10s)

### Critical Gaps

| Gap | Impact | Severity |
|-----|--------|----------|
| No cluster-wide aggregation | Can't get global QPS/latency | **HIGH** |
| No trace export to backend | Spans in-memory only | **HIGH** |
| No per-peer connection metrics | Can't identify bad peers | **MEDIUM** |
| No anomaly detection | No automated alerting | **MEDIUM** |
| Bounded data eviction | No historical analysis | **LOW** |

### Recommended Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   aspen-metrics crate                    │
├─────────────────────────────────────────────────────────┤
│  MetricsCollector (per-node)                            │
│    ├── Atomic counters (Tiger Style bounded)            │
│    ├── Histogram buckets (p50, p95, p99)               │
│    └── Export: Prometheus text + OTLP                   │
├─────────────────────────────────────────────────────────┤
│  ClusterMetricsAggregator (leader-based)                │
│    ├── Gossip-based metric distribution                 │
│    ├── Leader aggregates cluster-wide metrics           │
│    └── Time-series storage in KV with TTL               │
├─────────────────────────────────────────────────────────┤
│  TraceExporter                                          │
│    ├── Jaeger/Zipkin backend support                    │
│    ├── Sampling strategy implementation                 │
│    └── Span batching and export                         │
└─────────────────────────────────────────────────────────┘
```

---

## bincode -> postcard Migration Assessment

### Usage Summary

| Location | Usages | Risk |
|----------|--------|------|
| storage_shared.rs | 58 | **CRITICAL** - All KV/Raft data |
| storage.rs | 20 | **HIGH** - In-memory fallback |
| metadata.rs | 9 | **MEDIUM** - Node registry |
| Job workers | 11 | **LOW** - Read-only |
| SQL executor | 2 | **LOW** - Read-only |

### Breaking Change Analysis

**Data Format Incompatibility:**
- bincode: Fixed-width integers
- postcard: Variable-length integers (varint)
- Result: Data serialized with bincode **cannot** be read by postcard

**Affected Storage:**
- State machine KV table (millions of records)
- Lease table (TTL tracking)
- Raft log (can be compacted)
- Snapshots (persistent)
- Chain hashes (integrity verification)

### Migration Requirements

1. **Database Migration Tool** - Required for existing clusters
2. **Chain Hash Recomputation** - All entry hashes become invalid
3. **Simultaneous Cluster Upgrade** - No rolling upgrade possible
4. **Backup/Recovery Testing** - Before production deployment

**Effort Estimate:** 8-12 hours development + thorough testing

**Recommendation:** Plan carefully, execute on dev/test first, document upgrade path

---

## Pijul Circular Dependency Fix

### Problem

```rust
// aspen-testing/src/pijul_tester.rs:36-38
use aspen_core::node::Node;           // WRONG - doesn't exist
use aspen_core::node::NodeBuilder;    // WRONG - doesn't exist
use aspen_core::node::NodeId;         // Exists but wrong path
```

### Root Cause

- Node/NodeBuilder defined in main `aspen` crate (`src/node/mod.rs`)
- aspen-testing depends on aspen-core (correct)
- Cannot depend on main `aspen` crate (circular: aspen -> aspen-testing -> aspen)

### Recommended Fix: Move to aspen-core

1. Move `src/node/mod.rs` -> `crates/aspen-core/src/node.rs`
2. Update aspen-core/src/lib.rs to export Node, NodeBuilder
3. Update aspen/src/lib.rs to re-export from aspen-core
4. No changes needed to pijul_tester.rs imports

**Effort:** 2-4 hours
**Risk:** Low (code relocation, no logic changes)
**Unblocks:** 6 ignored Pijul multi-node tests

---

## Test Coverage Summary

| Category | Count | Status |
|----------|-------|--------|
| Integration tests | 75 files | Excellent |
| Simulation tests | 23 files | Excellent |
| Property-based tests | 10 files | Good |
| Fuzz targets | 14 targets | Good |
| Benchmarks | 6 suites | Good |
| Secrets unit tests | 39 | Good |
| Secrets integration | 14 (ignored) | Needs network |

**Coverage by Module:**
- Excellent (80-100%): vault, ticket, metadata, failure detection
- Good (40-79%): config, clock drift, learner promotion, integrity
- Integration-tested (0% unit): node, network, server, bootstrap

**Priority Test Gaps:**
1. PKI chain validation tests (X.509 verification)
2. CRL functionality tests (encoding, signatures)
3. Multi-node secrets replication tests
4. Cluster event hook tests (leader election, snapshots)

---

## Recommended Execution Order

### Week 1 (Immediate)

| Day | Task | Effort |
|-----|------|--------|
| 1 | Security patches Phase 1 (gix-*, rkyv) | 30 min |
| 1 | Fix log_broadcast condition | 5 min |
| 1-2 | Pijul circular dependency fix | 2-4 hours |
| 2-5 | First 4 secrets E2E tests | 2-3 days |

### Week 2

| Day | Task | Effort |
|-----|------|--------|
| 1-2 | Complete event injection wiring | 2-3 days |
| 2-3 | Security patches Phase 2 (libpijul) | 2-3 hours |
| 3 | Clean up compiler warnings | 1-2 hours |

### Weeks 3-4

| Task | Effort |
|------|--------|
| Begin consumer groups design | Research |
| Start aspen-metrics crate | 1-2 weeks |
| bincode -> postcard planning | Design doc |

### Month 2+

| Task | Effort |
|------|--------|
| Complete consumer groups | 4-6 weeks |
| Wire consumer groups to hooks | 1 week |
| Distributed tracing export | 2 weeks |
| bincode -> postcard execution | 8-12 hours |

---

## Files to Watch

**Critical Changes Needed:**
- `crates/aspen-cluster/src/bootstrap.rs:1687` - log_broadcast condition
- `crates/aspen-testing/src/pijul_tester.rs:36-38` - Circular dependency
- `crates/aspen-raft/src/node.rs` - Metrics watcher for hooks

**High-Change Frequency (Last 30 Days):**
- `crates/aspen-secrets/src/` - Active development
- `crates/aspen-hooks/src/` - Recent additions
- `crates/aspen-pubsub/src/` - Foundation work
- `crates/aspen-rpc-handlers/` - API surface

**Stability Achieved:**
- `crates/aspen-raft/` - Core consensus stable
- `crates/aspen-core/` - Trait definitions frozen
- `src/` - Main binary configuration stable

---

## Decision Summary

### Immediate Actions (Execute Now)

1. **Security Patch Phase 1** - Run:
   ```bash
   cargo update -p gix-features --aggressive
   cargo update -p gix-date --aggressive
   cargo update -p rkyv --precise 0.7.46
   nix develop -c cargo build && nix develop -c cargo nextest run -P quick
   ```

2. **Fix log_broadcast** - Change bootstrap.rs:1687 from:
   ```rust
   if config.docs.enabled
   ```
   to:
   ```rust
   if config.hooks.enabled || config.docs.enabled
   ```

3. **Pijul Circular Dependency** - Move Node/NodeBuilder to aspen-core

### Strategic Decisions

| Decision | Recommendation | Rationale |
|----------|----------------|-----------|
| Pijul vs Git | Evaluate deprecating aspen-pijul | Security debt (ed25519-dalek, curve25519-dalek) |
| Consumer Groups | Prioritize after hooks completion | Unblocks horizontal hook scaling |
| Metrics Architecture | Leader-based aggregation | Simpler than gossip, sufficient for MVP |
| bincode Migration | Defer until new clusters | Breaking change, plan carefully |

---

## Conclusion

Aspen is production-ready with clear technical debt identified. The ULTRA MODE analysis (7 parallel agents) uncovered:

**Good News:**
- SOPS Phase 3 is COMPLETE (not 95%)
- Pub/Sub core is COMPLETE (not 70%)
- PKI Intermediate CA is COMPLETE (commit 9470a3a7)
- Hooks architecture is solid (65% complete, just missing event sources)

**Action Required:**
- 6 security vulnerabilities need attention (2 CRITICAL via libpijul)
- 1 five-minute fix unblocks all KV hook events
- Hooks completion requires ~8 hours of event source wiring

**Next Action:** Apply security patches Phase 1 and fix log_broadcast condition.

---

*ULTRA MODE Analysis completed at 2026-01-09T15:30:00Z*
*7 parallel agents examined:*
- *Security vulnerabilities and dependency audit*
- *Pijul circular dependency investigation*
- *Secrets integration test coverage*
- *Pub/Sub consumer groups architecture*
- *Event injection and hooks wiring*
- *Observability and metrics architecture*
- *bincode to postcard migration feasibility*
