# What Next: ULTRA Mode Analysis

**Date**: 2026-01-08
**Analysis Type**: Maximum Capability Deep Analysis
**Status**: Strategic Roadmap

---

## Executive Summary

Based on comprehensive codebase exploration, recent commit history, decision documents, TODO analysis, and test coverage review, here is the prioritized "What Next" roadmap for Aspen.

**Key Finding**: Aspen has production-ready core infrastructure (~21,000 LOC, 350+ tests). Recent development has focused on higher-level orchestration features (pub/sub, hooks, command scheduling). The immediate priority is **completing integration of already-built components**.

---

## Current State Assessment

### Completed Infrastructure (Production-Ready)

| Component | Status | LOC | Tests |
|-----------|--------|-----|-------|
| Raft Consensus | Complete | ~6,500 | 100+ |
| Storage (Redb) | Complete | ~2,000 | 50+ |
| Iroh P2P Transport | Complete | ~2,500 | 40+ |
| Cluster Coordination | Complete | ~2,900 | 60+ |
| Forge (Git Bridge) | Complete | ~2,300 | 30+ |
| Jobs/Scheduling | Complete | ~2,500 | 40+ |
| CLI | Complete | ~1,500 | 20+ |

### Recently Completed (Integration Pending)

| Component | Status | LOC | Tests | Next Step |
|-----------|--------|-----|-------|-----------|
| aspen-hooks | Core Complete | 3,443 | 50 | NodeConfig + Bootstrap Integration |
| aspen-pubsub | Core Complete | ~1,200 | 20+ | Consumer Groups |
| ShellCommandWorker | Complete | ~500 | 10+ | Registration in worker pool |

---

## Priority 1: Complete Hooks Integration (CRITICAL)

**Why**: The `aspen-hooks` crate is fully implemented but not wired into the system. This blocks event-driven workflows.

**Files to Modify**:

```
1. crates/aspen-cluster/src/config.rs
   - Add: pub hooks: HooksConfig

2. crates/aspen-cluster/src/bootstrap_simple.rs
   - Create HookService from config
   - Spawn subscription tasks
   - Wire to KeyValueStore

3. crates/aspen-raft/src/storage_shared.rs
   - Inject hook event publishing in apply_to_state_machine()

4. crates/aspen-rpc-handlers/src/handlers/hooks.rs (NEW)
   - HookList, HookGetMetrics, HookTrigger endpoints

5. crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs (NEW)
   - aspen hook list/metrics/trigger commands
```

**Estimated Effort**: 14-22 hours

**Dependencies**: None (all prerequisites complete)

---

## Priority 2: Wire ShellCommandWorker (HIGH)

**Why**: Command scheduling infrastructure exists (75-80% complete) but lacks the worker that actually executes commands. This is the single missing piece for cron-style automation.

**Status**: ShellCommandWorker was recently added (commit 4dbb26ee) with:

- Capability-based authentication
- Timeout handling (SIGTERM/SIGKILL escalation)
- Output capture with size limits
- Environment filtering

**Remaining Work**:

1. Register `shell_command` job type in worker service
2. Add integration tests
3. Update CLI with `aspen schedule` subcommand

**Estimated Effort**: 1-2 days

---

## Priority 3: Pub/Sub Consumer Groups (HIGH)

**Why**: The pub/sub layer exists but lacks consumer groups (competing consumers). This is required for:

- Load-balanced event processing
- At-least-once delivery with checkpointing
- Hook execution distribution

**Architecture** (from decision doc):

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

## Priority 4: Integration Tests for Recent Features (HIGH)

**Why**: Several features have unit tests but lack integration tests:

| Feature | Unit Tests | Integration Tests |
|---------|------------|-------------------|
| aspen-hooks | 50 | 0 |
| aspen-pubsub | 20+ | 8 (ignored - need network) |
| ShellCommandWorker | 10+ | 0 |
| Forge Git Bridge | 30+ | 5 (ignored) |

**Test Files to Create/Enable**:

```
tests/hooks_integration_test.rs (NEW)
tests/shell_command_worker_test.rs (NEW)
Enable: tests/pubsub_integration_test.rs (currently ignored)
Enable: tests/forge_integration_test.rs (if exists)
```

**Estimated Effort**: 1 week

---

## Priority 5: Address Key TODOs (MEDIUM)

### Critical TODOs (in Aspen code, not openraft)

| Location | TODO | Priority |
|----------|------|----------|
| `crates/aspen-hooks/src/handlers.rs:373` | Cross-cluster forwarding | Medium |
| `crates/aspen-rpc-handlers/src/handlers/blob.rs:437` | Direct blob deletion | Low |
| `crates/aspen-jobs/src/event_store.rs:596` | Event upcasting for schema migration | Low |
| `crates/aspen-gossip/src/discovery.rs:352` | Topology version comparison | Medium |
| `crates/aspen-raft/src/storage.rs:1609` | Transaction support for in-memory SM | Low |

### Test Infrastructure TODOs

| Location | Description |
|----------|-------------|
| `tests/pubsub_integration_test.rs:54` | Add EventStream tests with StreamExt |
| `tests/pubsub_integration_test.rs:711` | CLI pub/sub command tests |
| `docs/forge.md:102-104` | Integration tests, property tests for Forge |

---

## Priority 6: Security Hardening (MEDIUM)

**From Command Scheduling Gap Analysis**:

1. **Command Allowlist** - Configuration option to restrict which commands can be scheduled
2. **Environment Filtering** - Prevent leaking secrets via env vars
3. **Resource Limits** - CPU/memory/disk limits for shell commands
4. **Audit Logging** - Track all command executions

**Implementation Location**: `crates/aspen-jobs/src/workers/shell_command.rs`

---

## Priority 7: Layer Architecture (FUTURE)

**From layer-architecture.md** - Consider implementing when needs arise:

1. **Tuple Encoding** (~500 LOC) - Type-safe, order-preserving key encoding
2. **Subspaces** (~200 LOC) - Namespace isolation for multi-tenancy
3. **Secondary Indexes** (~400 LOC) - Client-managed indexes with OCC guarantees

**Trigger Conditions**:

- Multi-tenant data isolation needed
- Query patterns require secondary indexes
- Integer/UUID ordering becomes problematic

---

## Architectural Gaps Identified

### Missing Features

| Gap | Impact | Solution |
|-----|--------|----------|
| Consumer Groups | Can't scale event processing | Priority 3 |
| Progress Notifications | Inefficient reconnection | Add bookmark events |
| Rate Limiting | Handler overload possible | Kubernetes dual rate limiter pattern |
| Watch Enumeration | No visibility into active hooks | Add HookList RPC |
| Backpressure | No flow control when handlers slow | Add backpressure mechanism |

### Test Gaps

| Test Type | Current | Needed |
|-----------|---------|--------|
| Unit Tests | Comprehensive | Good |
| Integration Tests | Limited (many ignored) | Enable network tests in CI |
| Madsim Simulation | Good coverage | Add hook simulation tests |
| Property Tests | Limited | Add proptest for pub/sub ordering |

---

## Recommended Execution Order

### Week 1

1. **Priority 1**: Hooks integration (NodeConfig + Bootstrap) - 2-4 hours
2. **Priority 1**: Hooks event publishing in state machine - 4-6 hours
3. **Priority 2**: Wire ShellCommandWorker to worker pool - 2-4 hours

### Week 2

1. **Priority 1**: Hooks RPC handlers + CLI commands - 4-6 hours
2. **Priority 4**: Integration tests for hooks - 4-6 hours
3. **Priority 4**: Integration tests for ShellCommandWorker - 4-6 hours

### Week 3

1. **Priority 3**: Pub/Sub Consumer Groups - start implementation
2. **Priority 4**: Enable/fix ignored integration tests

### Week 4+

1. **Priority 3**: Complete Consumer Groups
2. **Priority 5**: Address critical TODOs
3. **Priority 6**: Security hardening

---

## Success Metrics

After completing Priorities 1-4:

| Metric | Target |
|--------|--------|
| Hook events firing on KV changes | Working |
| Shell commands schedulable via CLI | Working |
| `aspen hook list/metrics` | Working |
| `aspen schedule add` | Working |
| Integration test coverage | +20 tests |
| Ignored tests | Reduced by 50% |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Hook blocking consensus | Low | Critical | Async spawn, timeout enforcement |
| Shell command injection | Medium | Critical | Command allowlist, input validation |
| Test flakiness from network tests | Medium | Medium | Use madsim for deterministic tests |
| Integration complexity | Medium | Medium | Incremental integration, test each step |

---

## Conclusion

Aspen's foundation is solid. The immediate focus should be on **integration** rather than new feature development:

1. **Wire the hooks system** - It's built, just not connected
2. **Register the shell worker** - Same situation
3. **Enable/write integration tests** - Validate the integrations
4. **Consumer groups** - Final piece for scalable event processing

This is approximately 4-6 weeks of focused work to complete the orchestration layer that makes Aspen more than "just" a distributed KV store.

---

## Quick Reference: Key Files

| Task | File |
|------|------|
| Hooks config | `crates/aspen-cluster/src/config.rs` |
| Hooks bootstrap | `crates/aspen-cluster/src/bootstrap_simple.rs` |
| Event publishing | `crates/aspen-raft/src/storage_shared.rs` |
| Shell worker | `crates/aspen-jobs/src/workers/shell_command.rs` |
| Pub/sub | `crates/aspen-pubsub/src/` |
| Integration tests | `tests/` |
