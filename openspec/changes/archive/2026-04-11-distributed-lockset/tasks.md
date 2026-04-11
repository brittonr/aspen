## 1. Verified core and data model

- [x] 1.1 Add a bounded `LockSet` request / guard model in `crates/aspen-coordination` with canonical member normalization and per-resource fencing token tracking
- [x] 1.2 Add pure helpers under `crates/aspen-coordination/src/verified/` for lock-set normalization, duplicate detection, per-member token computation, and set-wide backoff / renewal decisions
- [x] 1.3 Add Verus coverage or equivalent proof-linked invariants for atomic all-or-nothing acquisition and per-resource fencing token monotonicity

## 2. Coordination implementation

- [x] 2.1 Implement `try_acquire`, retrying `acquire`, `renew`, and `release` for distributed lock sets using one atomic conditional write per set operation
- [x] 2.2 Add bounded validation and actionable errors for duplicate members, oversized sets, and blocking live holders
  - Revalidated 2026-04-11: `try_acquire` preserves duplicate-member, oversized-set, generic, and partial-metadata failures as errors; only full live-holder contention metadata (`blocked_member` + `blocked_holder`) returns a blocked result.
- [x] 2.3 Add metrics and tracing for lock-set acquisition attempts, conflicts, renewals, releases, and takeover after expiry

## 3. Client and RPC surface

- [x] 3.1 Add client API request / response types for lock-set acquire, try_acquire, renew, and release operations
- [x] 3.2 Implement coordination RPC handlers and `aspen-client` wrappers for remote lock-set guards
  - Revalidated 2026-04-11: remote `try_acquire` exposes `blocked_member` / `blocked_holder` only when both fields describe live contention, while non-contention and partial-metadata failures stay actionable errors.
- [x] 3.3 Document canonical member ordering and guard semantics for downstream adopters in coordination-facing docs

## 4. Testing and adoption proof

- [x] 4.1 Add unit and property tests covering canonicalization, duplicate rejection, per-resource token monotonicity, and all-or-nothing contention behavior
- [x] 4.2 Add integration tests that exercise overlapping lock sets, expiry takeover, and renew / release through the remote client API
- [x] 4.3 Demonstrate one higher-level adopter or end-to-end scenario using `LockSet` to coordinate a compound operation without partial acquisition
