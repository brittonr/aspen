# aspen-hooks Integration Roadmap

**Date**: 2026-01-08
**Status**: Phase 1 Complete, Integration Pending

## Executive Summary

The `aspen-hooks` crate has been fully implemented with 3,443 lines of code across 10 source files, 50 passing tests, and 0 clippy warnings. The core hook system is production-ready but requires integration with the broader Aspen ecosystem.

## Completed Work

### Phase 1: Core Crate Structure (COMPLETE)

- `crates/aspen-hooks/Cargo.toml` - Dependencies configured
- `crates/aspen-hooks/src/lib.rs` - Public API exports
- `crates/aspen-hooks/src/constants.rs` - Tiger Style bounds
- `crates/aspen-hooks/src/error.rs` - Snafu error types
- `crates/aspen-hooks/src/config.rs` - HooksConfig, HookHandlerConfig, ExecutionMode
- `crates/aspen-hooks/src/publisher.rs` - HookPublisher wrapping RaftPublisher
- `crates/aspen-hooks/src/handlers.rs` - HookHandler trait + InProcess, Shell, Forward handlers
- `crates/aspen-hooks/src/worker.rs` - HookJobWorker implementing aspen-jobs Worker trait
- `crates/aspen-hooks/src/service.rs` - HookService with pattern matching and dispatch
- `crates/aspen-hooks/src/metrics.rs` - ExecutionMetrics with per-handler tracking

### Workspace Integration (COMPLETE)

- Added to workspace members in root Cargo.toml
- Added to workspace.dependencies
- Added as dependency of main aspen crate

## Remaining Integration Work

### Priority 1: NodeConfig Integration (CRITICAL)

**File**: `crates/aspen-cluster/src/config.rs`

Add HooksConfig to NodeConfig struct:

```rust
/// Hook system configuration
#[serde(default)]
pub hooks: HooksConfig,
```

**Rationale**: Without NodeConfig integration, hooks cannot be configured via TOML.

### Priority 2: Bootstrap Integration (CRITICAL)

**File**: `crates/aspen-cluster/src/bootstrap_simple.rs`

Tasks:

1. Create HookService from NodeConfig.hooks
2. Spawn HookService subscription tasks
3. Wire up to existing KeyValueStore
4. Register HookJobWorker with WorkerPool (if job manager exists)

**Architectural Decision**: HookService should be created AFTER Raft is initialized but BEFORE accepting client connections.

### Priority 3: Event Publishing Integration (HIGH)

**Files**:

- `crates/aspen-raft/src/storage_shared.rs` - KV operations
- `crates/aspen-raft/src/node.rs` - Cluster events

Inject hook event publishing at:

- `apply_to_state_machine()` - After KV write/delete commits
- Raft metrics changes - Leader election, membership changes
- Snapshot operations - Creation/installation

**Critical Consideration**: Hook publishing must NOT block the Raft consensus path. Use async spawn for event publishing.

### Priority 4: Integration Tests (HIGH)

**File**: `tests/hooks_integration_test.rs` (new)

Test scenarios:

- Single-node hook service initialization
- Direct mode handler execution
- Job mode handler submission
- Pattern matching (wildcards)
- Multi-node event propagation
- Shell handler with env variables
- Concurrency limit enforcement
- Handler timeout enforcement

### Priority 5: RPC Handler Integration (MEDIUM)

**Files**:

- `crates/aspen-rpc-handlers/src/handlers/hooks.rs` (new)
- `crates/aspen-rpc-handlers/src/context.rs` (add hook_service)
- `crates/aspen-rpc-handlers/src/handlers/mod.rs` (add module)
- `crates/aspen-rpc-handlers/src/registry.rs` (register handler)

RPC endpoints needed:

- `HookList` - List configured handlers
- `HookGetMetrics` - Get execution metrics
- `HookTrigger` - Manual trigger for testing

### Priority 6: CLI Commands (MEDIUM)

**Files**:

- `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs` (new)
- `crates/aspen-cli/src/bin/aspen-cli/commands/mod.rs` (add module)
- `crates/aspen-cli/src/bin/aspen-cli/cli.rs` (add Hook variant)

Commands:

- `aspen hook list` - List handlers
- `aspen hook metrics` - Show execution metrics
- `aspen hook trigger` - Manual trigger

## Architectural Decisions

### 1. Synchronous vs Async Hook Execution

**Decision**: Hooks are ALWAYS async and non-blocking to consensus.

**Rationale**: Based on research into FoundationDB, etcd, and CockroachDB patterns:

- FoundationDB: Watches are async notifications, never block commits
- etcd: Watch notifications are sent after commit, not inline
- CockroachDB CDC: Change feeds are async, decoupled from consensus

### 2. Feature Flag Strategy

**Decision**: Hooks will NOT have a feature flag initially.

**Rationale**:

- aspen-hooks has minimal dependencies (reuses existing pubsub/jobs)
- No heavy external dependencies to gate
- Similar to aspen-pubsub and aspen-jobs (always compiled)

### 3. Event Publishing Location

**Decision**: Publish events from state machine application, not from RPC handlers.

**Rationale**:

- Events should only fire for COMMITTED operations
- State machine application is the linearization point
- Avoids duplicate events on retries

### 4. Configuration Scope

**Decision**: HooksConfig is per-node, not cluster-wide.

**Rationale**:

- Different nodes may have different handlers (e.g., only leader runs analytics)
- Shell handlers are inherently node-local
- Cluster-wide config can be layered on top later

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Hook blocking consensus | Low | Critical | Async spawn, timeout enforcement |
| Memory exhaustion from events | Medium | High | Bounded queues, semaphore limits |
| Handler failure cascades | Medium | Medium | DLQ for job mode, error isolation |
| Cross-cluster forwarding failures | Medium | Low | Retry logic, circuit breaker |

## Testing Strategy

1. **Unit Tests**: Already complete in aspen-hooks (50 tests)
2. **Integration Tests**: Use NodeBuilder pattern from pubsub tests
3. **Simulation Tests**: Use madsim for deterministic failure injection
4. **Property Tests**: Proptest for configuration validation

## Timeline Recommendation

| Phase | Estimated Effort | Dependencies |
|-------|------------------|--------------|
| NodeConfig + Bootstrap | 2-4 hours | None |
| Event Publishing | 4-6 hours | Phase 1 |
| Integration Tests | 4-6 hours | Phase 2 |
| RPC + CLI | 4-6 hours | Phase 3 |

Total: ~14-22 hours of focused work

## Next Actions

1. **Immediate**: Add HooksConfig to NodeConfig
2. **Then**: Integrate HookService in bootstrap
3. **Then**: Add event publishing to state machine
4. **Then**: Write integration tests
5. **Finally**: Add RPC/CLI support

## References

- Original plan: `/home/brittonr/git/aspen/events.md`
- Crate location: `/home/brittonr/git/aspen/crates/aspen-hooks/`
- Similar integration: aspen-dns, aspen-forge patterns
