# Dead-Letter Redrive and Recovery

Dead-letter handling is a controlled transition in a bounded delivery state machine, not a second execution engine or an unlimited archive. This article assumes the claim and token model described by the [coordination delivery contract](../../coordination-delivery.md). It concentrates on exhausted work, authorized re-entry, retention, and uncertain storage outcomes. See the [Technical companion](../README.md) for the adjacent claim and actor lifecycle discussions.

## A dead letter preserves causal context

Under the reviewed profile, poison classification or exhausted attempts can move an in-flight item into the dead-letter collection. The [completion transitions](../../../crates/molten-core/src/coordination_delivery/transition/completion.rs) append an attempt outcome before constructing the dead-letter entry. The entry records its item, entry tick, cycle, attempts in that cycle, total recorded attempts, and reason. Payload remains referenced rather than copied into delivery state.

The [support helper](../../../crates/molten-core/src/coordination_delivery/transition/support.rs) checks dead-letter capacity before insertion and computes the retention deadline with checked arithmetic. If dead-letter capacity is exhausted, the enclosing pure transition returns a denial preserving the input state. A bounded DLQ is therefore not a promise that any failed item can always be relocated immediately. Capacity is part of admissibility, and the caller must not interpret a denied relocation as successful quarantine.

Attempt history and current eligibility answer different questions. History describes prior terminal attempt observations; ready or in-flight membership describes what can happen next. A new delivery cycle can change eligibility without erasing the history of why the item was quarantined.

## Redrive creates a new cycle, not a clean past

The [redrive transition](../../../crates/molten-core/src/coordination_delivery/transition/retention.rs) requires the exact policy redrive authority reference and available ready capacity. It removes the selected dead-letter entry, increments its cycle with checked arithmetic, assigns a fresh enqueue sequence, and makes it ready at the current admitted logical tick. Attempts within the new cycle start at zero. Prior attempt history is retained.

The fresh enqueue sequence is important under strict FIFO. Redriven work re-enters queue ordering at a new position instead of recovering its historical place ahead of unrelated ready work. Redrive also emits cancellation of the dead-letter retention timer associated with the old entry. The timer intent is a consequence of an accepted transition, not the authority that allowed re-entry.

Redrive does not certify that a poison cause was repaired. An operator can have the right authority while the referenced payload still triggers the same application failure. Evidence of remediation, payload correctness, and admission of subsequent worker effects belong to their own boundaries.

## Cleanup is independently authorized

Dead-letter cleanup uses the distinct retention authority reference. Its `through_tick` cannot exceed the request's logical tick. It selects entries whose checked retention expiry is at or before that cutoff and denies the request when none qualify. For removed entries it emits retention-timer cancellation intents.

This is removal from the delivery state's dead-letter collection. The inspected helper does not delete referenced content bytes or clear the separate attempt history. Describing DLQ cleanup as content garbage collection or complete historical erasure would overstate its implementation. Similarly, a timer notification or an old receipt cannot replace the retention authority check.

## Recovery separates planning from durable outcome

The [delivery shell](../../../src/coordination_delivery/service.rs) loads the published queue state, checks the expected state reference and revision, then invokes the pure planner. Only an applied transition reaches compare-and-commit. Stale expected state produces a preserving outcome rather than a speculative write.

If compare-and-commit returns an unknown disposition or an error marked outcome-unknown, the shell reads the queue once. Equality with the exact planned published state yields `AppliedAfterReconciliation`; equality with the expected prior state yields `NotAppliedAfterReconciliation`; another observed state yields `Unknown`. The code does not blindly issue a second commit.

Those classifications should be read with the receipt's currentness, durability, and engine-epoch fields rather than replacing them. The service retains an original commit observation when one exists. A reconciliation label is not license to manufacture a stronger storage observation than the adapter supplied.

Timer and status effects occur only after a confirmed commit classification. Their failures remain explicit without rewriting the durable queue transition. This ordering prevents “timer service unavailable” from being interpreted as “the redrive did not happen,” but it also means operators must inspect both durable state and follow-up observations.

## Worked redrive uncertainty scenario

Suppose, illustratively, item `P` is dead-lettered in cycle one after exhausting the reviewed profile's two attempts. An authorized redrive request at logical tick 200 plans cycle two, a new enqueue sequence, and immediate eligibility. The store applies the new state but its response is lost.

The service reads back exactly the planned published state and classifies the redrive as applied after reconciliation. It does not redrive again and increment to cycle three. If retention-timer cancellation then fails, the durable item remains ready in cycle two; the failed timer observation is a separate reconciliation concern.

Now consider the alternative where readback finds a third state, perhaps reflecting a different accepted transition. The shell cannot identify that value as either the exact expected state or exact planned state, so the result remains unknown. Guessing “probably applied” and resubmitting would destroy the distinction the reconciliation algorithm preserves. None of these storage observations determines whether an earlier worker external effect occurred.

## Verification and limits

The existing [uncertainty tests](../../../src/coordination_delivery/tests/uncertain.rs) inspect unknown-before and unknown-after commit handling, timer failure after durable commit, stale observations, and stale expected state. They are source evidence for those scenarios; they were not executed for this article. Suggested additional review follows a redriven item's cycle, sequence, retained attempts, and old-token rejection, then exercises DLQ capacity and retention cutoff boundaries.

A dead letter does not imply safe payload, a redrive does not authorize worker effects, and successful reconciliation does not prove global ordering or a correct external system. Receipts remain canonical outcome records, not mutation capabilities. At-least-once delivery, bounded history, and explicit uncertainty do not establish exactly-once effects or production readiness.

## Sources

- [Coordination delivery extension](../../coordination-delivery.md)
- [Addressable actor unknown-effect boundary](../../addressable-actor-runtime.md)
- [Dead-letter entry and attempt helpers](../../../crates/molten-core/src/coordination_delivery/transition/support.rs)
- [Failure and exhaustion transitions](../../../crates/molten-core/src/coordination_delivery/transition/completion.rs)
- [Redrive and retention transitions](../../../crates/molten-core/src/coordination_delivery/transition/retention.rs)
- [Delivery commit and readback shell](../../../src/coordination_delivery/service.rs)
- [Uncertain-commit and follow-up failure tests](../../../src/coordination_delivery/tests/uncertain.rs)
- [Technical companion](../README.md)
