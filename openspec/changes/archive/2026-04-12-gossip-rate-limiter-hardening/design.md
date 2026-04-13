## Context

`crates/aspen-cluster/src/gossip/rate_limiter.rs` currently keeps bounded per-peer state and supports injected timestamps for tests, but it still mutates the shared global bucket before it knows whether the per-peer bucket will reject the same message. That lets a single noisy peer drain global burst capacity with traffic that should already be denied locally.

The same module also re-implements token refill and admission math that overlaps with `crates/aspen-cluster/src/verified/rate_limiter.rs`. Aspen's Tiger Style rules prefer one deterministic functional core with a thin imperative shell, especially for time-sensitive logic that must behave the same in runtime code and injected-time tests.

Constraints:

- keep the existing configured limits and bounded-memory behavior
- preserve monotonic handling for backward timestamps
- avoid new dependencies
- keep runtime integration simple for `gossip::discovery::lifecycle`

## Goals / Non-Goals

**Goals:**

- prevent per-peer rejections from consuming global rate-limit budget
- make runtime and injected-time paths share one deterministic transition model
- keep per-peer limiter state opaque enough that invariants stay module-owned
- add regression coverage for the audited failure mode and the clock-monotonicity edge cases

**Non-Goals:**

- redesign gossip discovery itself
- change default rate/burst constants
- introduce a new persistence layer or cross-node rate coordination
- rewrite unrelated gossip modules

## Decisions

### 1. Use atomic two-tier admission decisions

**Choice:** compute the per-peer and global bucket outcomes from current state and the provided timestamp, then commit state changes only for the dimensions that should actually advance for that result.

**Rationale:** the audited bug comes from mutating shared state too early. A single admission decision must decide both allow/deny and which state transitions are valid.

**Alternative considered:** checking the per-peer bucket first and then the global bucket. This avoids one failure mode, but it still mutates one side before the full decision is known and leaves asymmetry in the rejection paths.

**Implementation:** add pure helpers that return next-state data for global and per-peer buckets from explicit inputs, then let the runtime shell write those results back into `GossipRateLimiter`.

### 2. Make `src/verified/rate_limiter.rs` the source of truth

**Choice:** extend the verified rate-limiter module so refill, consume, and bounded-eviction decisions needed by the runtime wrapper live in deterministic helpers.

**Rationale:** Aspen already documents `verified/` modules as the functional core. Gossip limiter logic uses explicit time and bounded state, so it fits this pattern well. Sharing the same helper path between runtime code and injected-time tests reduces FCIS drift.

**Alternative considered:** keep the current stateful runtime math and only patch the global-budget bug locally. That would fix the immediate bug but leave duplicate logic and make future audits harder.

**Implementation:** add or extend pure transition helpers for bucket refill/consume, monotonic timestamp handling, and capacity/eviction decisions. The shell wrapper remains responsible for `Instant` storage and `HashMap` mutation.

### 3. Encapsulate invariant-bearing peer entries

**Choice:** keep `PeerRateEntry` and any direct mutable fields private unless a narrower visibility is required for tests.

**Rationale:** `last_access` monotonicity and bucket consistency are invariants owned by the limiter. Callers should not be able to rewrite them arbitrarily.

**Alternative considered:** leave structs public because the module is small. That keeps the audit smell in place and makes future invariants easier to bypass.

**Implementation:** reduce visibility, keep tests in the module or add minimal test-only accessors if required.

## Risks / Trade-offs

- **Floating-point transition drift** → Reuse one helper path and add parity tests so runtime and injected-time paths agree on allow/deny decisions.
- **Refactor broadens scope into gossip discovery code** → Keep the public `GossipRateLimiter` API stable so call sites do not need large changes.
- **Over-specifying internal details** → Put architectural constraints in this design doc and only add spec requirements for behavior that must remain stable.

## Migration Plan

No external migration is required. The implementation is an internal hardening change within `aspen-cluster`. Rollback is a normal code revert if the regression tests expose incompatibilities.

## Open Questions

- Whether the pure helper surface should model a full combined rate-limit decision enum or smaller bucket-level transitions composed in the shell.
- Whether any lightweight Verus spec should be added now or left for a follow-up once the runtime refactor lands.
