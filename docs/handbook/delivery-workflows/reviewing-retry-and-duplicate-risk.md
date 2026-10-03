# Reviewing retry and duplicate risk

Mode: Review checklist

Use this checklist before approving a delivery integration change or accepting an incident proposal to repeat work. The review output is a decision with evidence and unresolved boundaries, not a general assertion that delivery is idempotent. Return to the [Handbook](../README.md); the [claims companion](../../technical/replication/delivery-claims-acks-and-retry.md) explains the mechanics underlying these questions.

This checklist is source-checked, not an executed verification report. Ask for actual artifacts from the change under review. Checked-in tests describe available scenarios but do not establish that a candidate or deployment passed them. A delivery receipt never substitutes for authority, semantic correctness, or release evidence.

## Establish the operation being reviewed

- [ ] Does the proposal say whether it changes diagnostic deduplication, coordination transitions, or worker effects? Record each owner explicitly. The [diagnostic library](../../../src/delivery/idempotency.rs) and [coordination shell](../../../src/coordination_delivery/service.rs) are not one atomic external-effect system.
- [ ] Are scope, producer, consumer, sequence, intent, payload reference, and ordered policy/evidence references preserved in the evidence packet? Require canonical records, not a screenshot of a short summary.
- [ ] Is identity derived from canonical Preserves with BLAKE3 rather than Rust layout, host paths, wall time, or an informal payload label? If scope is supplied by name, show the exact name-derived binding; if explicit, show which reference won.
- [ ] Does the proposed fix retain the original diagnostic coordinates? A new sequence or scope chosen only to obtain `first` should block approval until its business semantics are justified independently.

## Inspect the diagnostic-to-effect boundary

- [ ] Where is the first diagnostic record committed relative to starting the business effect? The [first-decision implementation](../../../src/delivery/parts/idempotency/p006/body.rs) persists before returning. Require a caller-specific account of crashes before effect start, after possible effect, and before terminal evidence.
- [ ] Does a duplicate reuse the prior recorded semantic result without claiming the result was newly measured? Require both the prior receipt binding and application-owned completion evidence when completion matters.
- [ ] Are changed payload, policy, and evidence inputs treated as comparison changes rather than harmless retries? The [decision law](../../../src/delivery/parts/idempotency/p001/body.rs) compares operation identity, payload, and evidence vector before window arithmetic.
- [ ] Is `retry` understood as a gap-policy decision with side effects suppressed? Require the missing sequence/window evidence; do not accept “retry receipt present” as authorization to repeat an uncertain external action.
- [ ] If concurrency guarantees are claimed, is there consumer-visible evidence beyond a sequential fixture? Reads/classification and the first-decision write are separate source steps. This observation is a review boundary, not a reproduced concurrency bug or proof of an unsupported guarantee.

## Review claims and token currentness

- [ ] Does every completion use the exact currently active token, not just item identity or consumer name? Compare attempt, cycle, fencing token, deadline, generation, epoch, and policy.
- [ ] Does the proposal account for lease extension replacing the token reference? Demand evidence that a delayed old-token completion cannot complete the extended claim.
- [ ] Are completion and expiry tested against admitted logical time? The [active-token helper](../../../crates/molten-core/src/coordination_delivery/transition/support.rs) rejects completion at or after the deadline, while expiry is separately authorized. Wall-clock timeout is insufficient.
- [ ] Is wrong-owner completion denied unless exact delegated completion authority is supplied? Require the authority source, not a previously successful receipt.
- [ ] Under strict FIFO, does the reviewer inspect the earliest item's eligibility rather than bypassing it to improve apparent throughput? Delayed-head behavior belongs to the selected policy, not a broken worker by itself.

## Review failure, redrive, and retention

- [ ] Is the failure class supported by the admitted policy? Require the actual nack/expiry request and its outcome, not only a worker log.
- [ ] Can a full DLQ or ready collection deny the transition while preserving original state? Review the [planner's all-or-preserve result](../../../crates/molten-core/src/coordination_delivery/transition.rs), not intermediate mutations in a helper.
- [ ] Does redrive use exact redrive authority, a new cycle and enqueue sequence, and retained prior attempts? Require before/planned-after/confirmed-state evidence using the [redrive implementation](../../../crates/molten-core/src/coordination_delivery/transition/retention.rs).
- [ ] Is remediation evidence separate from requeue permission? Neither redrive nor resetting the new-cycle attempt counter establishes payload safety.
- [ ] Is retention independently authorized and bounded by logical cutoff? Do not approve deletion of evidence or referenced content as a shortcut to resolving delivery uncertainty.

## Review unknown outcomes and close the decision

- [ ] Does unknown commit handling compare exact expected and planned published states without blind commit replay? Attach readback evidence and retain currentness, durability, and engine epoch.
- [ ] Are durable commit, timer follow-up, status publication, and worker completion reported separately? The [uncertainty fixtures](../../../src/coordination_delivery/tests/uncertain.rs) contain relevant distinctions; identify which were actually exercised for this change.
- [ ] For addressable actors, is a durable semantic-event reference present before planned acknowledgement, and is unresolved external-effect state handled under the [actor contract](../../addressable-actor-runtime.md)? Explicit resolution must not be presented as automatic retry authority.

Worked rejection case: a proposal says “timer failed, so rerun the claim and worker.” If evidence instead shows an applied claim with a failed timer reference, reject that proposal. The queue mutation already has a confirmed classification, and the worker effect may be independently uncertain. Accept a narrower reconciliation decision only when it names the failing port boundary, preserves the active token and history, and supplies the responsible owner's evidence.

## Sources

- [Handbook](../README.md)
- [Coordination delivery contract](../../coordination-delivery.md)
- [Claims and retry companion](../../technical/replication/delivery-claims-acks-and-retry.md)
- [Addressable actor completion and uncertainty](../../addressable-actor-runtime.md)
- [Diagnostic generated trace](../../../src/delivery/parts/idempotency/tests/m000/p000/body.rs)
- [Coordination uncertainty fixtures](../../../src/coordination_delivery/tests/uncertain.rs)
