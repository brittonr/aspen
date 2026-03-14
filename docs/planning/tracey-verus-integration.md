# Tracey + Verus Integration Plan

Status: DRAFT — spec for review before implementation.

## Problem

Aspen has 89 Verus spec files across 19 crates and 64 verified modules across 18 crates. The invariants are documented as prose comments in `verus/lib.rs` files (e.g., "LOCK-1: Fencing Token Monotonicity"). None of this is tracked by Tracey. There's no `.config/tracey/config.styx`, no requirement docs with `r[...]` markers, and no `r[impl ...]` / `r[depends ...]` / `r[verify ...]` annotations in source.

Separately, the `src/verified/` subdirectory pattern adds indirection. Pure functions live in `src/verified/lock.rs` and get called as `verified::is_lock_expired(...)` from shell code in `src/lock.rs`. The verus-tracey combined pattern puts exec functions with `ensures` clauses directly in `src/`, eliminating the extra module layer.

## Goals

1. Every formally verified invariant becomes a Tracey requirement with `r[...]` marker.
2. Every production function, spec function, and proof function gets the appropriate Tracey annotation (`impl`, `depends`, `verify`).
3. `src/verified/` directories dissolve — pure functions move into the main `src/` module tree.
4. `tracey query uncovered` and `tracey query untested` run in CI alongside `nix run .#verify-verus`.

## Inventory

### Crates with both `verus/` and `src/verified/` (9 crates — full integration)

| Crate | Verus Specs | Verified Modules | Notes |
|---|---|---|---|
| aspen-coordination | 27 | 20 | Largest. Locks, elections, queues, barriers, fencing, semaphores, rate limiters, registries, workers, strategies |
| aspen-raft | 13 | 10 | Chain integrity, batching, TTL, storage, snapshots, membership |
| aspen-core | 6 | 1 | HLC, tuple encoding, directory ops, scan/pagination |
| aspen-cluster | 5 | 4 | Gossip, identity, trust, blob announcements |
| aspen-deploy | 2 | 1 | Quorum, overflow |
| aspen-federation | 2 | 2 | Fork detection, quorum |
| aspen-forge | 3 | 3 | Ref store, repo identity, signed objects |
| aspen-jobs | 2 | 5 | Priority, saga |
| aspen-kv-branch | 1 | 1 | Scan merge |
| aspen-secrets | 1 | 1 | MAC |
| aspen-transport | 2 | 2 | Connections, streams |

### Crates with `verus/` only (8 crates — annotations only, no module move)

aspen-auth, aspen-automerge, aspen-blob, aspen-cache, aspen-client-api, aspen-sharding, aspen-snix, aspen-ticket

### Crates with `src/verified/` only (6 crates — module move + test-based `r[verify]`)

aspen-ci-core, aspen-ci, aspen-net, aspen-raft-network, aspen-redb-storage, aspen-rpc-handlers

## Tracey Configuration

```styx
// .config/tracey/config.styx
specs (
  // --- Phase 1: Core triad ---
  {
    name coordination-requirements
    include (docs/requirements/coordination.md)
    impls (
      {
        name rust
        include (crates/aspen-coordination/src/**/*.rs crates/aspen-coordination/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-coordination/tests/**/*.rs)
      }
    )
  }
  {
    name raft-requirements
    include (docs/requirements/raft.md)
    impls (
      {
        name rust
        include (crates/aspen-raft/src/**/*.rs crates/aspen-raft/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-raft/tests/**/*.rs)
      }
    )
  }
  {
    name core-requirements
    include (docs/requirements/core.md)
    impls (
      {
        name rust
        include (crates/aspen-core/src/**/*.rs crates/aspen-core/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-core/tests/**/*.rs)
      }
    )
  }
  // --- Phase 2: Remaining verus-enabled crates ---
  {
    name cluster-requirements
    include (docs/requirements/cluster.md)
    impls (
      {
        name rust
        include (crates/aspen-cluster/src/**/*.rs crates/aspen-cluster/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-cluster/tests/**/*.rs)
      }
    )
  }
  {
    name auth-requirements
    include (docs/requirements/auth.md)
    impls (
      {
        name rust
        include (crates/aspen-auth/src/**/*.rs crates/aspen-auth/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-auth/tests/**/*.rs)
      }
    )
  }
  {
    name transport-requirements
    include (docs/requirements/transport.md)
    impls (
      {
        name rust
        include (crates/aspen-transport/src/**/*.rs crates/aspen-transport/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-transport/tests/**/*.rs)
      }
    )
  }
  {
    name forge-requirements
    include (docs/requirements/forge.md)
    impls (
      {
        name rust
        include (crates/aspen-forge/src/**/*.rs crates/aspen-forge/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-forge/tests/**/*.rs)
      }
    )
  }
  {
    name deploy-requirements
    include (docs/requirements/deploy.md)
    impls (
      {
        name rust
        include (crates/aspen-deploy/src/**/*.rs crates/aspen-deploy/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-deploy/tests/**/*.rs)
      }
    )
  }
  {
    name federation-requirements
    include (docs/requirements/federation.md)
    impls (
      {
        name rust
        include (crates/aspen-federation/src/**/*.rs crates/aspen-federation/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-federation/tests/**/*.rs)
      }
    )
  }
  {
    name jobs-requirements
    include (docs/requirements/jobs.md)
    impls (
      {
        name rust
        include (crates/aspen-jobs/src/**/*.rs crates/aspen-jobs/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-jobs/tests/**/*.rs)
      }
    )
  }
  {
    name sharding-requirements
    include (docs/requirements/sharding.md)
    impls (
      {
        name rust
        include (crates/aspen-sharding/src/**/*.rs crates/aspen-sharding/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-sharding/tests/**/*.rs)
      }
    )
  }
  {
    name blob-requirements
    include (docs/requirements/blob.md)
    impls (
      {
        name rust
        include (crates/aspen-blob/src/**/*.rs crates/aspen-blob/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-blob/tests/**/*.rs)
      }
    )
  }
  {
    name automerge-requirements
    include (docs/requirements/automerge.md)
    impls (
      {
        name rust
        include (crates/aspen-automerge/src/**/*.rs crates/aspen-automerge/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-automerge/tests/**/*.rs)
      }
    )
  }
  {
    name ticket-requirements
    include (docs/requirements/ticket.md)
    impls (
      {
        name rust
        include (crates/aspen-ticket/src/**/*.rs crates/aspen-ticket/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-ticket/tests/**/*.rs)
      }
    )
  }
  {
    name snix-requirements
    include (docs/requirements/snix.md)
    impls (
      {
        name rust
        include (crates/aspen-snix/src/**/*.rs crates/aspen-snix/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-snix/tests/**/*.rs)
      }
    )
  }
  {
    name cache-requirements
    include (docs/requirements/cache.md)
    impls (
      {
        name rust
        include (crates/aspen-cache/src/**/*.rs crates/aspen-cache/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-cache/tests/**/*.rs)
      }
    )
  }
  {
    name client-api-requirements
    include (docs/requirements/client-api.md)
    impls (
      {
        name rust
        include (crates/aspen-client-api/src/**/*.rs crates/aspen-client-api/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-client-api/tests/**/*.rs)
      }
    )
  }
  {
    name kv-branch-requirements
    include (docs/requirements/kv-branch.md)
    impls (
      {
        name rust
        include (crates/aspen-kv-branch/src/**/*.rs crates/aspen-kv-branch/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-kv-branch/tests/**/*.rs)
      }
    )
  }
  {
    name secrets-requirements
    include (docs/requirements/secrets.md)
    impls (
      {
        name rust
        include (crates/aspen-secrets/src/**/*.rs crates/aspen-secrets/verus/**/*.rs)
        exclude (target/**)
        test_include (crates/aspen-secrets/tests/**/*.rs)
      }
    )
  }
)
```

## Requirement ID Scheme

Full dotted names. Crate prefix matches the crate name minus `aspen-`.

```
coordination.lock.fencing-monotonicity
coordination.lock.mutual-exclusion
coordination.lock.ttl-expiration
coordination.election.token-monotonicity
coordination.election.single-leader
raft.chain.integrity
raft.storage.single-fsync
core.hlc.monotonicity
```

## `src/verified/` Dissolution

### What changes

Each crate that has `src/verified/` gets its pure functions moved into the main module tree. The `verified::` module path disappears. Callers update their imports.

### Strategy per crate

**When the verified module mirrors a shell module** (common case):

- `src/verified/lock.rs` functions move into `src/lock.rs` (or a `src/lock/pure.rs` submodule if `lock.rs` is already large)
- The function signatures stay identical. Only the module path changes.

**When the verified module is standalone** (e.g., `src/verified/fencing.rs` with no `src/fencing.rs`):

- The file moves to `src/fencing.rs` and gets declared as `pub mod fencing` in `lib.rs`.

### Migration steps (per crate)

1. Identify each `src/verified/foo.rs` and its corresponding `src/foo.rs` (if any).
2. Move pure functions into the target module.
3. Delete `src/verified/foo.rs`.
4. Update `src/verified/mod.rs` (remove the submodule).
5. Update all `use crate::verified::foo::*` and `verified::foo(...)` call sites.
6. Update `pub use verified::*` re-exports in `lib.rs`.
7. When `src/verified/mod.rs` is empty, delete the directory and remove `pub mod verified` from `lib.rs`.
8. Run `cargo build` and `cargo nextest run` for the crate.

### Import pattern changes

```rust
// Before
use crate::verified::lock::is_lock_expired;
use crate::verified::lock::compute_next_fencing_token;
// or
let expired = verified::is_lock_expired(deadline, now);

// After
use crate::lock::is_lock_expired;
use crate::lock::compute_next_fencing_token;
// or
let expired = lock::is_lock_expired(deadline, now);
```

### Tracey annotation placement after move

The pure function now lives in `src/lock.rs` with an `r[impl ...]` annotation:

```rust
// r[impl coordination.lock.fencing-monotonicity]
pub fn compute_next_fencing_token(current_entry: Option<&LockEntry>) -> u64 {
    match current_entry {
        Some(entry) => entry.fencing_token.saturating_add(1),
        None => 1,
    }
}
```

The verus spec function in `verus/lock_state_spec.rs` gets `r[depends ...]`:

```rust
// r[depends coordination.lock.fencing-monotonicity]
pub open spec fn fencing_token_monotonic(pre: LockState, post: LockState) -> bool {
    post.max_fencing_token_issued >= pre.max_fencing_token_issued
}
```

The verus proof function gets `r[verify ...]`:

```rust
// r[verify coordination.lock.fencing-monotonicity]
proof fn prove_fencing_monotonicity(pre: LockState, post: LockState)
    requires ...
    ensures fencing_token_monotonic(pre, post)
{ }
```

## Phases

### Phase 1: Pilot — `aspen-coordination`

The biggest crate. 27 verus specs, 20 verified modules, ~250 pure functions.

1. Write `docs/requirements/coordination.md` — extract invariants from `crates/aspen-coordination/verus/lib.rs` into `r[...]` markers.
2. Create `.config/tracey/config.styx` (coordination entry only to start).
3. Dissolve `src/verified/` — move pure functions into main modules.
4. Add `r[impl ...]` annotations to production functions.
5. Add `r[depends ...]` to verus spec functions.
6. Add `r[verify ...]` to verus proof functions.
7. Run `tracey query status` — fix gaps.
8. Run full test suite — everything passes.

### Phase 2: `aspen-raft` and `aspen-core`

Same pattern. These are the other two crates with substantial verification.

### Phase 3: Remaining 8 crates with both `verus/` and `src/verified/`

cluster, deploy, federation, forge, jobs, kv-branch, secrets, transport.

### Phase 4: 8 crates with `verus/` only (no `src/verified/` to dissolve)

auth, automerge, blob, cache, client-api, sharding, snix, ticket.

Annotations only — add `r[depends ...]` and `r[verify ...]` to spec/proof functions, `r[impl ...]` to whatever production functions the specs describe.

### Phase 5: 6 crates with `src/verified/` only (no `verus/` specs)

ci-core, ci, net, raft-network, redb-storage, rpc-handlers.

Dissolve `src/verified/`. Add `r[impl ...]` to moved functions. Use `r[verify ...]` on `#[test]` functions since there are no proof functions.

### Phase 6: CI integration

- Add `tracey query uncovered --exit-code` and `tracey query untested --exit-code` to the verification pipeline.
- Combine with `nix run .#verify-verus` in a single `scripts/check-coverage.sh`.
- Gate CI: Verus passes AND Tracey reports zero uncovered/untested.

## Sample Requirement Doc: `coordination.md` (excerpt)

```markdown
# Coordination Requirements

## Distributed Lock

r[coordination.lock.fencing-monotonicity]
Fencing tokens MUST strictly increase across lock acquisitions on the same key.

r[coordination.lock.mutual-exclusion]
At most one holder MUST hold a given lock at any point in time.

r[coordination.lock.ttl-expiration]
A lock whose TTL has expired MUST be treated as released and be reacquirable.

r[coordination.lock.deadline-computation]
Lock deadline MUST be computed as acquisition time plus TTL using saturating arithmetic.

## Sequence Generator

r[coordination.sequence.uniqueness]
No two calls to next() on the same sequence MUST return the same value.

r[coordination.sequence.monotonicity]
Each value returned by next() MUST be strictly greater than the previous value.

r[coordination.sequence.batch-reservation]
Batch reservation MUST atomically reserve a contiguous range of values.

## Atomic Counter

r[coordination.counter.saturating-arithmetic]
Counter operations MUST use saturating arithmetic — no overflow or underflow panics.

r[coordination.counter.cas-atomicity]
Compare-and-swap on counters MUST be atomic with respect to concurrent operations.

## Leader Election

r[coordination.election.single-leader]
At most one leader MUST exist for a given election key at any point in time.

r[coordination.election.token-monotonicity]
Election tokens MUST strictly increase across leadership transitions.

r[coordination.election.valid-transitions]
Leadership state transitions MUST follow the defined state machine (follower → candidate → leader → follower).

## Read-Write Lock

r[coordination.rwlock.mutual-exclusion]
A write lock and any read lock MUST NOT be held simultaneously on the same key.

r[coordination.rwlock.reader-bounded]
The number of concurrent readers MUST NOT exceed the configured maximum.

## Semaphore

r[coordination.semaphore.permit-bounded]
Acquired permits MUST NOT exceed total permits configured for the semaphore.

## Distributed Queue

r[coordination.queue.visibility-timeout]
A dequeued message MUST NOT be visible to other consumers until its visibility timeout expires.

r[coordination.queue.dlq-routing]
A message that exceeds max delivery attempts MUST be moved to the dead letter queue.

r[coordination.queue.dedup-window]
Duplicate messages within the deduplication window MUST be silently dropped.

## Barrier

r[coordination.barrier.phase-ordering]
Barrier phases MUST advance monotonically — no phase regression.

r[coordination.barrier.completion]
A barrier MUST complete when all registered participants have arrived.

r[coordination.barrier.deadlock-detection]
Stalled barriers MUST be detectable via deadlock check within bounded time.

## Rate Limiter

r[coordination.rate-limiter.token-bounded]
Available tokens MUST NOT exceed the configured burst capacity.

r[coordination.rate-limiter.replenishment]
Tokens MUST replenish at the configured rate per interval.

## Service Registry

r[coordination.registry.ttl-expiration]
A service instance whose TTL has expired MUST be treated as deregistered.

r[coordination.registry.heartbeat-liveness]
A service instance MUST be considered live only if its last heartbeat is within the timeout window.

## Fencing

r[coordination.fencing.split-brain-detection]
Split-brain conditions MUST be detectable by comparing fencing tokens across partitions.

r[coordination.fencing.quorum]
Operations requiring quorum MUST verify that a strict majority of nodes agree.

## Worker Coordination

r[coordination.worker.liveness]
A worker MUST be considered alive only if its last heartbeat is within the timeout window.

r[coordination.worker.capacity-bounded]
Task assignment MUST NOT exceed a worker's declared capacity.

r[coordination.worker.work-stealing]
Work stealing MUST only occur from overloaded workers to underloaded workers.
```

## Decisions

1. **Clean break on `verified::` paths.** No backward compat alias. All `use crate::verified::`, `verified::foo()` call sites, and `pub use verified as pure` re-exports get rewritten. The `pub mod verified` declaration is removed from `lib.rs` once empty.
2. **Pure functions go inline.** Even when `src/lock.rs` already has async shell code, the pure functions move into the same file. No `src/lock/pure.rs` submodules.
3. **Start unversioned.** Requirements use bare `r[coordination.lock.fencing-monotonicity]` with no `+N` suffix. Add versioning later when requirements start changing.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| `src/verified/` dissolution breaks downstream imports | Clean break — update all call sites in the same commit. `cargo build` catches stragglers. |
| Large diff on coordination crate | Do the module move and annotation as separate commits. Module move first, annotations second. |
| Requirement extraction misses invariants | Cross-reference verus spec files against the requirement doc. `tracey query uncovered` catches gaps mechanically. |
| Verus specs reference functions by old path | Verus specs are standalone (they redefine the functions). No path dependency on `src/`. |
