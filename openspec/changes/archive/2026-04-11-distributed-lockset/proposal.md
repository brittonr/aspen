## Why

Aspen has a strong single-resource distributed lock with fencing tokens, but multi-resource coordination still falls back to ad hoc ordering and repeated single-lock acquisition. That leaves higher-level workflows to manage partial acquisition, crash cleanup, and overlapping contention themselves.

## What Changes

- Add a distributed `LockSet` primitive that acquires a bounded set of named lock resources atomically through Raft consensus
- Canonicalize member ordering inside the primitive so callers requesting the same set in different orders converge on one acquisition plan
- Preserve per-resource fencing tokens so stale holders can still be rejected after TTL expiry or takeover
- Support `try_acquire`, retrying `acquire`, `renew`, and `release` for the full set as one guard
- Expose the primitive through coordination libraries and client RPCs so higher-level systems can coordinate compound operations without bespoke lock choreography

## Capabilities

### New Capabilities

- `distributed-lockset`: Atomic acquisition, renewal, release, and fencing of multi-resource distributed lock sets

### Modified Capabilities

## Impact

- `crates/aspen-coordination/` — new lock-set primitive, verified helpers, and tests
- `crates/aspen-client-api/` and coordination RPC handlers — wire types and handlers for lock-set operations
- `crates/aspen-client/` — client wrapper and guard type for remote lock sets
- Higher-level orchestration crates (`aspen-ci`, `aspen-forge`, `aspen-deploy`, `aspen-cluster`) can adopt the primitive for compound resource claims after the API lands
