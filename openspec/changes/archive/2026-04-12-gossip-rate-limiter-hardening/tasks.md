## 1. Specification and deterministic core

- [x] 1.1 Add a delta spec for `gossip-rate-limiter` covering atomic shared-budget accounting, deterministic timestamp-driven transitions, and encapsulated limiter state.
- [x] 1.2 Extend `crates/aspen-cluster/src/verified/rate_limiter.rs` with the pure transition helpers needed for gossip limiter admission decisions and monotonic time handling.

## 2. Runtime hardening

- [x] 2.1 Refactor `crates/aspen-cluster/src/gossip/rate_limiter.rs` to use the verified helper path and avoid consuming global budget on per-peer rejection.
- [x] 2.2 Reduce visibility of invariant-bearing peer-entry state so callers must go through `GossipRateLimiter` APIs.

## 3. Regression coverage

- [x] 3.1 Add deterministic tests proving a noisy peer cannot exhaust global budget with traffic that is already rejected by the per-peer limiter.
- [x] 3.2 Add deterministic parity tests for backward timestamps and LRU monotonicity across the verified core and runtime wrapper.
- [x] 3.3 Run targeted `aspen-cluster` checks/tests that cover the refactor and save implementation evidence when the change is worked.
