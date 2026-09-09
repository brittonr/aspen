# Command reservation design

## Boundary

The pure core owns reservation validation, planning, batch admission, and capacity accounting over in-memory authoritative state. The shell owns effect execution, storage commit, and retries. No new port is needed; the existing effects surface already covers decided effects.

## Proposed decision

The reservation command carries the job, the logical operation identity, the resource requirements, and the expected generation. The transition evaluates it atomically against current authoritative state: generation must be current, capacity must satisfy requirements, and the outcome is either a committed reservation with an allocation or a typed rejection that preserves state. There is no intermediate client-visible read-then-write window.

Batch admission is bounded before evaluation. A batch declares an item count, a byte count, and a maximum waiting time; admission rejects batches that exceed any limit. Each item keeps its own identity and result: one rejected item does not fail or admit its neighbors, and the set of items committed atomically is exactly the set the contract declares atomic — batching does not enlarge atomicity.

Capacity accounting distinguishes available, reserved, and allocated quantities as constrained transitions, following the pending/post/void shape. Every transition preserves the invariant that these quantities do not drift, including on rejection paths.

Lease expiry changes bookkeeping only. An expired lease marks its reservation stale; the capacity behind it becomes reusable only through a transition whose precondition includes an enforcement or termination fact — an observed stop, a fencing takeover with termination authority, or a contract that declares the work non-external. Where the contract cannot support reuse, capacity stays counted against the stale reservation and the shortfall is visible in operator status.

## Compatibility and replay

Existing single-reservation and completion traces keep their meanings. New command traces join replay cohorts with deterministic outcomes: same state and same command produce the same decision. Receipt fixtures gain the batch-limit and capacity-invariant fields. `resume-blocked-scheduler-runnables` capacity composition and `bind-scheduler-completions-to-lease-epoch` token checks are regression controls that must keep passing.

## Coordination and order

This change shapes the scheduler API; the oracles change measures its cost. No circular dependency: this change lands the shape and limits, novelty and counter comparisons stay with the oracles package. `bound-terminal-scheduler-retention` and `enforce-scheduler-queue-bounds-on-yield` keep their existing queue-bound scopes.

## Validation and ownership

Scheduler maintainers own transitions, batch admission, capacity types, docs, and fixtures. Baseline existing scheduler and capacity tests before edits. Cover contended interleavings, stale generation, every batch-limit boundary, per-item isolation, expiry with and without enforcement facts, and capacity-invariant checks after every path including rejections. Run focused Octet and Clippy checks, workspace tests, relevant Nix checks, and required Cairn gates. This plan grants no implementation permission and claims no throughput or performance improvement.
