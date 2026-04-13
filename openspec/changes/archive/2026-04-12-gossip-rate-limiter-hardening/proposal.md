## Why

The Tiger Style audit of `crates/aspen-cluster/src/gossip/rate_limiter.rs` found one concrete safety bug and two structural weaknesses:

- rejected per-peer traffic still burns the shared global budget
- the runtime limiter duplicates transition logic that already belongs in `src/verified/`
- invariant-bearing per-peer state is more mutable than necessary

This change turns those findings into a durable implementation plan before the bug and the FCIS drift spread further.

## What Changes

- **Atomic gossip admission accounting**: Change gossip rate-limit evaluation so a per-peer rejection does not consume shared global budget.
- **Verified-core alignment**: Move bucket transition rules and monotonic time handling behind deterministic helpers in `crates/aspen-cluster/src/verified/rate_limiter.rs`, then make the runtime wrapper delegate to that core.
- **State encapsulation**: Narrow the visibility of per-peer rate-limit state so callers cannot bypass limiter invariants.
- **Regression coverage**: Add targeted tests for noisy-peer starvation, backward timestamps, and runtime-vs-injected-time parity.

## Capabilities

### New Capabilities

- `gossip-rate-limiter`: Defines atomic shared-budget accounting, deterministic bucket transitions, and encapsulated gossip limiter state.

### Modified Capabilities

## Impact

- **Files**: `crates/aspen-cluster/src/gossip/rate_limiter.rs`, `crates/aspen-cluster/src/verified/rate_limiter.rs`, `crates/aspen-cluster/src/gossip/discovery/lifecycle.rs` integration call sites if needed, and new/updated tests in `aspen-cluster`.
- **APIs**: `GossipRateLimiter` keeps its public check/new entry points, but invariant-bearing helper structs may become private or less mutable.
- **Dependencies**: No new dependencies.
- **Testing**: Add deterministic regression tests for per-peer rejection accounting and backward-time monotonicity; run targeted `aspen-cluster` checks/tests during implementation.
