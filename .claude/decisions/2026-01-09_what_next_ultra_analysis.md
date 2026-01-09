# What Next: ULTRA Mode Analysis

**Date**: 2026-01-09
**Analysis Type**: Maximum Capability Deep Analysis
**Status**: Strategic Roadmap - Post-Hooks Integration

---

## Executive Summary

The Aspen project has made significant progress since the last analysis (2026-01-08). The hooks system integration is now **COMPLETE**, with full wiring into NodeConfig, bootstrap, and event bridges for Raft log, blob, and docs events. The immediate priorities have shifted.

**Key Finding**: The v3 branch represents a massive feature release with 376k+ lines of changes. The focus should now shift to:

1. **Stabilization** - Enable and run ignored integration tests
2. **Consumer Groups** - Final piece for scalable event processing
3. **Feature Gating** - The `shell-worker` feature flag needs proper wiring
4. **V3 Release Preparation** - Consider merging to main

---

## Current State Assessment

### Hooks System (COMPLETED - Updated from 2026-01-08)

The roadmap from 2026-01-08 identified these as "pending integration":

| Task | Status | Evidence |
|------|--------|----------|
| NodeConfig integration | COMPLETE | `crates/aspen-cluster/src/config.rs:203` - `pub hooks: HooksConfig` |
| Bootstrap integration | COMPLETE | `crates/aspen-cluster/src/bootstrap.rs:1752` - HookService creation |
| Raft event bridge | COMPLETE | `crates/aspen-cluster/src/hooks_bridge.rs` - 395 lines |
| Blob event bridge | COMPLETE | `crates/aspen-cluster/src/blob_bridge.rs` - 297 lines |
| Docs event bridge | COMPLETE | `crates/aspen-cluster/src/docs_bridge.rs` - 240 lines |
| CLI commands | COMPLETE | `aspen-cli hooks list/metrics/trigger/create-url/trigger-url` |
| Hook URL tickets | COMPLETE | `crates/aspen-hooks/src/ticket.rs` - AspenHookTicket |
| Hook client library | COMPLETE | `crates/aspen-hook-client/` |

The hooks system is now **fully integrated** and operational.

### Infrastructure Status

| Component | LOC | Tests | Status |
|-----------|-----|-------|--------|
| Raft Consensus | ~6,500 | 100+ | Production-ready |
| Storage (Redb) | ~2,000 | 50+ | Production-ready |
| Iroh P2P Transport | ~2,500 | 40+ | Production-ready |
| Cluster Coordination | ~2,900+ | 60+ | Production-ready |
| Hooks System | ~4,500 | 63 | **Newly integrated** |
| Pub/Sub | ~1,200 | 20+ | Core complete, needs consumer groups |
| Jobs System | ~2,500 | 40+ | Core complete |
| Shell Worker | ~800 | 10+ | Behind feature flag |
| Forge (Git Bridge) | ~2,300 | 30+ | Complete |
| CLI | ~1,500 | 20+ | Complete |

### Feature Flags

| Feature | Default | Description | Status |
|---------|---------|-------------|--------|
| `sql` | Yes | DataFusion SQL queries | Production |
| `dns` | Yes | DNS record management | Production |
| `tui` | No | Terminal UI | Production |
| `fuse` | No | FUSE filesystem | Production |
| `global-discovery` | No | Mainline DHT | Production |
| `shell-worker` | **No** | Shell command execution | **Needs wiring** |
| `fuzzing` | No | Fuzz testing support | Dev-only |
| `bolero` | No | Property-based testing | Dev-only |

---

## Priority 1: Enable Shell Worker Feature (COMPLETED)

**Status**: COMPLETED on 2026-01-09

**What Was Done**:

1. Added `shell-worker` to default features in `Cargo.toml:262`
2. Verified the worker was already fully wired in `aspen-node.rs:854-878`
3. Confirmed build compiles and clippy passes with 0 warnings
4. Verified tests pass

**How It Works**:

- The `shell-worker` feature is now enabled by default
- Worker only registers when `--enable-token-auth` is passed (security gate)
- Requires `CapabilityToken` with `ShellExecute` capability for each job
- Logs warning if token auth is not enabled

**To Use**:

```bash
# Start node with token authentication
aspen-node --enable-token-auth --trusted-root-key <hex-key>

# Submit shell command job with auth token
# Token must have ShellExecute capability for the command
```

**Estimated Effort**: 4-8 hours → **Actual: 1 hour** (was already wired, just needed feature flag)

---

## Priority 2: Consumer Groups for Pub/Sub (HIGH)

**Why**: The pub/sub layer exists but lacks consumer groups (competing consumers). This blocks:

- Load-balanced event processing
- At-least-once delivery with checkpointing
- Scalable hook handler distribution

**Architecture** (from decision docs):

```
__pubsub/groups/{group}/cursor      -> u64 (committed cursor)
__pubsub/groups/{group}/members/{node} -> {heartbeat, partitions}
```

**Implementation**:

1. Add `ConsumerGroupCoordinator` in `crates/aspen-pubsub/src/consumer_group.rs`
2. Partition assignment via consistent hashing
3. Cursor checkpointing through Raft

**Estimated Effort**: 1 week

---

## Priority 3: Integration Test Enablement (HIGH)

**Why**: 55+ integration tests are marked `#[ignore]` due to network requirements. These need a testing strategy.

### Ignored Test Analysis

| Test File | Ignored Tests | Reason |
|-----------|---------------|--------|
| `node_builder_integration.rs` | 12 | Network access |
| `multi_node_cluster_test.rs` | 4 | Network access |
| `job_integration_test.rs` | 8 | Network access |
| `hooks_integration_test.rs` | 6 | Network access |
| `pubsub_integration_test.rs` | 8 | Network access |
| `shell_command_worker_test.rs` | 6 | Network + feature flag |
| `dns_integration_test.rs` | 2 | Network access |
| `gossip_e2e_integration.rs` | 2 | Network access |
| `pijul_multi_node_test.rs` | 6 | Circular dependency |

### Testing Strategy Options

1. **Script-based integration tests** (existing)
   - `scripts/aspen-cluster-raft-smoke.sh` - Works outside Nix sandbox
   - `scripts/kitty-hooks-test.sh` - Comprehensive hooks testing
   - Pro: Already working, CI-friendly
   - Con: Not in Rust test harness

2. **Enable in CI with network** (recommended for some)
   - Run ignored tests in a separate CI job with network access
   - Use `cargo nextest run --run-ignored ignored-only`

3. **Convert to madsim tests** (high effort)
   - Rewrite network tests to use deterministic simulation
   - Pro: Deterministic, faster
   - Con: Major rewrite effort

**Recommendation**: Keep critical path tests as scripts, convert key scenarios to madsim.

---

## Priority 4: V3 Release Preparation (MEDIUM)

**Why**: The `v3` branch has 376,454 lines of changes across 1,502 files. This is a major release.

### Pre-merge Checklist

- [ ] All quick tests passing
- [ ] Script-based integration tests passing
- [ ] Clippy clean (0 warnings)
- [ ] Documentation updated (CLAUDE.md reflects current state)
- [ ] Decision documents organized
- [ ] Changelog written

### Breaking Changes to Document

1. Actor architecture removed (Dec 2025)
2. HTTP API removed (Iroh-only now)
3. New hooks system
4. New pub/sub system
5. New jobs system with workers

---

## Priority 5: Address Key TODOs (MEDIUM)

### Critical TODOs in Aspen Code

| Location | TODO | Priority |
|----------|------|----------|
| `handlers.rs:373` | Cross-cluster hook forwarding | Medium |
| `blob.rs:437` | Direct blob deletion | Low |
| `event_store.rs:596` | Event upcasting for schema migration | Low |
| `discovery.rs:352` | Topology version comparison | Medium |
| `storage.rs:1609` | Transaction support for in-memory SM | Low |

### Test Infrastructure TODOs

| Location | Description |
|----------|-------------|
| `pubsub_integration_test.rs:54` | Add EventStream tests with StreamExt |
| `pubsub_integration_test.rs:711` | CLI pub/sub command tests |

---

## Priority 6: Pijul Multi-Node Tests (LOW)

**Why**: 6 tests are ignored due to "circular dependency" issues.

**Location**: `tests/pijul_multi_node_test.rs`

**Issue**: The test module has a circular dependency with something else.

**Recommendation**: Investigate and fix, or remove if feature is deprecated.

---

## Architectural Recommendations

### 1. Shell Worker Feature Default

**Question**: Should `shell-worker` be enabled by default?

**Considerations**:

- Pro: Enables cron-style automation out of the box
- Con: Security implications (command execution)
- Recommendation: Keep disabled by default, document clearly

### 2. Integration Test CI Strategy

**Recommendation**: Create two-tier CI:

- Tier 1: Quick tests + madsim (runs in Nix sandbox)
- Tier 2: Full integration tests (runs with network access)

### 3. Consumer Groups Design

**Recommendation**: Follow Kafka consumer group semantics:

- Partitions assigned round-robin
- Heartbeat-based membership
- Offset commit through Raft (linearizable)

---

## Success Metrics for Next Sprint

| Metric | Target | Current |
|--------|--------|---------|
| Quick tests passing | 100% | ~350 |
| Integration scripts passing | 100% | Needs verification |
| Shell worker wired | Done | Not started |
| Consumer groups | Design doc | Not started |
| V3 merge readiness | Checklist complete | In progress |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Shell worker security issues | Medium | High | Feature flag, capability-based auth |
| Consumer group complexity | Medium | Medium | Follow Kafka patterns |
| V3 merge conflicts | Low | Medium | Branch is already ahead |
| Test coverage gaps | Medium | Medium | Script tests cover critical paths |

---

## Quick Reference: Key Files

| Task | File |
|------|------|
| Shell worker | `crates/aspen-jobs/src/workers/shell_command.rs` |
| Worker registration | `crates/aspen-cluster/src/bootstrap.rs` |
| Pub/sub | `crates/aspen-pubsub/src/` |
| Integration test scripts | `scripts/aspen-cluster-*.sh` |
| Hooks tests | `scripts/kitty-hooks-test.sh` |

---

## Recommended Execution Order

### This Week

1. Verify all quick tests pass
2. Run integration scripts to confirm hooks work end-to-end
3. Document `shell-worker` feature enablement

### Next Week

1. Wire ShellCommandWorker into worker pool (if feature enabled)
2. Design consumer groups architecture
3. Start V3 release notes

### Following Weeks

1. Implement consumer groups
2. Enable key integration tests in CI
3. Merge V3 to main
