# 6. Tiger Style Resource Bounds

**Status:** accepted

## Context

Distributed systems fail in production when resources are unbounded — unbounded queues cause OOM, unbounded scans cause timeouts, unbounded connections exhaust file descriptors. These failures are hard to reproduce in testing because test workloads are small.

Tiger Style is a set of coding principles (inspired by TigerBeetle) that emphasize explicit resource limits, assertion density, and deterministic behavior. The core idea: every data structure, loop, and queue must have a fixed upper bound.

## Decision

All resource limits are defined as constants in `crates/aspen-constants/src/` organized by domain:

- `raft.rs`: `MAX_BATCH_SIZE` (1,000 entries), `MAX_LOG_ENTRIES` (100,000)
- `network.rs`: `MAX_PEERS` (1,000), `MAX_CONCURRENT_CONNECTIONS` (500), timeouts (5s connect, 2s stream, 10s read)
- `api.rs`: `MAX_KEY_SIZE` (1 KB), `MAX_VALUE_SIZE` (1 MB), `MAX_SCAN_RESULTS` (10,000)
- `coordination.rs`: Lock TTLs, barrier participant limits
- `ci.rs`: Pipeline limits, job timeouts
- `plugin.rs`: WASM memory limits, execution timeouts

Code that processes external input must check against these limits and return errors (not panic) when exceeded.

Additional Tiger Style rules enforced in the codebase:

- No `.unwrap()`/`.expect()` in production code — use `?` with snafu `context()`
- No `panic!()`/`todo!()`/`unimplemented!()` in production code
- Functions under 70 lines
- Explicitly sized types (`u32`, `i64`) not `usize`
- Never hold locks across `.await` points
- Boolean prefixes: `is_`, `has_`, `should_`, `can_`

Alternatives considered:

- (+) No fixed limits: simpler code, flexible for all workloads
- (-) No fixed limits: unbounded resources cause production failures
- (+) Configurable limits: users tune for their workload
- (-) Configurable limits: adds complexity, misconfiguration risk, hard to test all combinations
- (+) Fixed constants: predictable, testable, no configuration surface
- (~) Fixed constants: some users may hit limits — but limits can be raised in future versions

## Consequences

- Resource exhaustion bugs are caught at compile-review time, not in production
- Constants are centralized — changing a limit is a one-line change with full-text search visibility
- Code review can verify bounded behavior by checking constant usage
- Tests can exercise boundary conditions (at-limit, over-limit) deterministically
- Naming conventions (`timeout_ms`, `size_bytes`) make units unambiguous in code review
