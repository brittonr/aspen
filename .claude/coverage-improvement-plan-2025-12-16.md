# Coverage Improvement Plan

Created: 2025-12-16T22:19:55-05:00
Author: Claude Code (ULTRA mode analysis)

## Current State

- **Total Coverage**: 27.97% (2,603 / 9,307 lines)
- **Threshold**: 25% minimum (currently meeting)
- **Test Files**: 66 integration tests, 350+ test functions
- **Test-to-Code Ratio**: 1.26:1 (excellent investment in testing)

## Analysis Summary

### Excellent Coverage (90%+)

- `cluster.metadata`: 97.98%
- `cluster.ticket`: 100%
- `api.vault`: 100%
- `raft.node_failure_detection`: 94.55%

### Good Coverage (60-89%)

- `raft.storage_validation`: 78.52%
- `raft.learner_promotion`: 68.97%
- `cluster.config`: 67.75%
- `raft.integrity`: 65.68%
- `raft.clock_drift_detection`: 75%

### Low/Zero Coverage (Priority Targets)

| Module | Current | Target | Lines | Priority |
|--------|---------|--------|-------|----------|
| `raft.types` | 18% | 50% | 204 | HIGH |
| `raft.supervisor` | 0% | 40% | 218 | HIGH |
| `raft.rpc` | 0% | 30% | 133 | HIGH |
| `cluster.gossip_discovery` | 19.55% | 40% | 443 | MEDIUM |
| `raft.connection_pool` | 0% | 20% | 542 | MEDIUM |

## Implementation Strategy

### Phase 1: Quick Wins (This Session)

Focus on modules that can be unit tested without complex mocking:

1. **raft/types.rs** (204 lines, 18% → 50%)
   - NodeId: FromStr, Display, ordering
   - AppRequest/AppResponse: serialization roundtrips
   - RaftMemberInfo: construction and display

2. **raft/supervisor.rs** (218 lines, 0% → 40%)
   - Restart counting logic
   - Backoff duration progression
   - Circuit breaker after MAX_RESTARTS

3. **raft/rpc.rs** (133 lines, 0% → 30%)
   - Already partially tested in server_rpc_test.rs
   - Add dedicated rpc.rs inline tests

4. **cluster/gossip_discovery.rs** (443 lines, 19.55% → 40%)
   - Message serialization
   - Malformed message handling
   - Already has some tests, extend coverage

### Phase 2: Mock-Dependent Tests (Future)

Modules requiring Iroh endpoint mocking:

- `raft.connection_pool`
- `cluster.mod` (IrohEndpointManager)
- `raft.network`

### Phase 3: Integration-Heavy Tests (Future)

Modules tested via integration tests:

- `raft.node` - via router tests
- `raft.server` - via server_rpc_test.rs
- `cluster.bootstrap` - via smoke tests

## Test Patterns to Follow

### Unit Test Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specific_behavior() {
        // Arrange
        let input = ...;
        // Act
        let result = function_under_test(input);
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Property-Based Test Pattern

```rust
proptest! {
    #[test]
    fn test_roundtrip(input in arbitrary_input()) {
        let serialized = serialize(&input);
        let deserialized = deserialize(&serialized)?;
        prop_assert_eq!(input, deserialized);
    }
}
```

## Success Criteria

1. Raise overall coverage from 27.97% to 32%+
2. All new tests pass with zero warnings
3. No regressions in existing tests
4. Update .coverage-baseline.toml with new targets

## Files to Modify

1. `src/raft/types.rs` - Add inline #[cfg(test)] module
2. `src/raft/supervisor.rs` - Add inline #[cfg(test)] module
3. `src/raft/rpc.rs` - Add inline #[cfg(test)] module
4. `src/cluster/gossip_discovery.rs` - Extend existing tests
5. `.coverage-baseline.toml` - Update targets after implementation

## Risk Mitigation

- Run `cargo nextest run` after each file modification
- Use `cargo check` for quick compilation verification
- Follow Tiger Style resource bounds in test assertions
- Ensure deterministic tests (no random, no sleep)
