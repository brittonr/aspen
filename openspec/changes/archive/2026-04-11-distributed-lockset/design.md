## Context

Aspen's current distributed lock API is centered on one resource key at a time. It provides fencing tokens, TTL-based recovery, and a retrying acquire path, which is enough for single-resource coordination. The missing piece is a first-class way to claim several related resources together.

Today, callers that need compound exclusion have to sort names themselves, acquire single locks one by one, and unwind partial success when a later lock is unavailable. That pattern is error-prone even when every caller follows the same convention, and it pushes contention policy into higher-level orchestration code instead of the coordination layer.

## Goals / Non-Goals

**Goals:**

- Provide a bounded, first-class distributed `LockSet` primitive for named resources
- Make acquisition all-or-nothing so callers never hold only a subset of the requested resources
- Preserve Aspen's existing fencing-token model on each locked resource
- Keep the design compatible with Raft-backed linearizability and existing KV transaction machinery
- Expose the primitive through local coordination libraries and remote client APIs

**Non-Goals:**

- Compile-time lock ordering or local mutex wrappers for in-process synchronization
- Fair queuing or starvation-free scheduling guarantees in v1
- Subset release, lock promotion, or downgrade semantics in v1
- Cross-cluster or federated lock ownership in v1

## Decisions

### Use runtime canonical ordering over resource names

`LockSet` will accept dynamic resource names, reject duplicate members as invalid input, and sort the validated member list by a stable total order before any storage operation. Aspen's coordination resources are largely runtime-discovered, so a Surelock-style type-level ordering model is not a good fit here. Canonicalization inside the primitive removes caller-specific ordering bugs while keeping the API ergonomic.

Alternative considered: require callers to pass pre-sorted keys and document the rule. Rejected because it weakens the guarantee and recreates the current footgun.

### Acquire the whole set with one atomic conditional write

The coordinator will read every requested lock entry, verify that each is absent, released, or expired, compute the next fencing token for each member, and then commit the full set with one conditional batch / transaction. If any member changes between read and write, the whole acquisition fails and retries from the top.

This keeps the semantics linearizable and avoids partial acquisition. It also maps directly onto Aspen's existing batch/transaction support instead of introducing a separate lock manager service.

Alternative considered: sequentially acquire sorted single locks. Rejected because it still creates partial success, more retries, and more crash cleanup even if it avoids circular wait.

### Reuse per-resource lock entries and add a lock-set guard

Each member resource will continue to store a lock entry with holder id, fencing token, acquisition time, TTL, and deadline. `LockSetGuard` will track the canonical member list plus the expected serialized entry for each member so `renew` and `release` can update the whole set conditionally.

This keeps compatibility with existing fencing semantics and lets future adopters use the same downstream token-checking patterns they already use for single locks.

Alternative considered: store one aggregate lock-set record plus per-resource pointers. Rejected for v1 because it adds a second index and complicates takeover / recovery logic without buying much.

### Bound lock-set size explicitly

Add a `MAX_LOCKSET_KEYS` constant and reject requests that exceed it. The initial value should be small enough to bound Raft transaction size and retry cost while still covering realistic orchestration bundles.

Alternative considered: unbounded vectors. Rejected because Aspen follows fixed resource bounds and unbounded lock sets would create pathological batch sizes.

### Match single-lock lifecycle semantics at the set level

`try_acquire` returns immediately, `acquire` retries with bounded backoff, `renew` extends all members together, and `release` transitions all members to released entries together. If any member is currently held by another live holder, the acquire attempt fails without mutating any key.

The error surface should report the blocking member key and holder so callers can log useful diagnostics.

### Expose lock-set operations through client RPCs

The primitive should not be library-only. Add coordination RPCs and `aspen-client` wrappers for acquire / try_acquire / renew / release so CI, Forge, deploy, and future orchestration layers can use the same behavior from remote clients.

Alternative considered: keep it server-local and defer remote APIs. Rejected because most Aspen orchestration entry points already coordinate via client RPCs.

## Risks / Trade-offs

- Large overlapping lock sets may increase retry churn under contention -> keep the bound small, add metrics for retries / conflicts, and encourage narrowly scoped resource bundles
- Set-wide renew and release mean one corrupted member entry can invalidate the whole guard -> guard stores exact expected values and returns actionable conflict errors instead of silently releasing a subset
- Reusing per-resource entries means adopters must choose stable resource names carefully -> document naming conventions and recommend prefix-based namespacing per subsystem
- The primitive prevents partial acquisition, not starvation -> keep fairness out of v1 and measure contention before designing a queueing policy

## Open Questions

- Should v1 use `ConditionalBatch` exact-value checks or the richer `Transaction` path for member validation?
- What initial `MAX_LOCKSET_KEYS` bound best balances orchestration needs against Raft write size?
- Do we want a dedicated metrics namespace (`aspen.lockset.*`) in the first implementation, or can it ride on existing coordination metrics initially?
