# Delivery Claims, Acks, and Retry

Coordination delivery records who currently holds bounded work and which transitions can follow that claim. It does not run the worker's external business operation. This article assumes logical event time, content references, and compare-and-commit storage. The [coordination delivery contract](../../coordination-delivery.md) remains authoritative; the [Technical companion](../README.md) connects this discussion to the other fabric mechanisms.

## Claiming is a state transition

`plan_delivery_transition` validates policy, manifest, current state, request, and admitted time profile before applying an operation. It constructs a prospective state and returns a denied transition preserving the original state if an operation fails or the result does not validate. The [transition coordinator](../../../crates/molten-core/src/coordination_delivery/transition.rs) therefore provides a deterministic all-or-preserve boundary around changes to several collections. Removing an item inside a helper does not mean a later capacity error loses that item from accepted state.

A [claim](../../../crates/molten-core/src/coordination_delivery/transition/queue.rs) selects one eligible ready item, increments its attempt within the current cycle, allocates a fencing token, derives the delivery identity, and inserts the item into in-flight state. Its token binds queue, item, consumer, attempt, cycle, fencing token, claim tick, visibility deadline, consistency epoch, service generation, and policy. These are the coordinates of a particular claim, not merely descriptive metadata attached to a reusable item identifier.

The planner produces a lease-expiry timer intent alongside the claim. The shell commits the state before executing follow-up timer work. Thus “claimed durably” and “expiry timer scheduled successfully” are different observations, even though both belong to handling the same request.

## Exact tokens fence completion

The completion helper retrieves the in-flight entry and compares the entire supplied token with its active token. Ack, nack, and lease extension require an unexpired claim. The requester must be the owning consumer unless it supplies the policy's exact delegated completion authority reference. These rules are implemented in [current-active validation](../../../crates/molten-core/src/coordination_delivery/transition/support.rs).

An extension is not a mutation that leaves an old token equivalent to the new one. The [completion transitions](../../../crates/molten-core/src/coordination_delivery/transition/completion.rs) derive a later deadline, recompute the token reference, replace the active token, and emit cancellation and scheduling intents for the old and new deadlines. A late ack carrying the old token is consequently a mismatch even if its worker believes it still owns the same item.

Ack removes in-flight state, appends an acknowledged attempt, and records completed delivery information. Nack first classifies the supplied failure against the admitted policy. Unsupported classes are denied. A retryable failure below the attempt limit moves the item to ready state with a future eligibility tick; poison or exhaustion can enter the bounded dead-letter collection. Neither path infers whether an external effect occurred before the worker reported failure.

## Logical time gives a precise boundary

The reviewed [Nickel profile](../../../config/coordination-delivery/profile.ncl) uses a ten-tick visibility timeout, two maximum attempts, a five-tick fixed retry delay, strict FIFO ordering, and no jitter. These are inspected profile values, not universal delivery constants or wall-clock durations.

Completion requires a request tick strictly earlier than the active deadline. Expiry instead requires the exact expiry authority reference and a request tick at or after that deadline. The same boundary cannot admit both unexpired completion and expiry under the same active token. A process timeout is not itself an admitted logical-time request and cannot bypass token or authority checks.

The retry deadline uses the accepted fabric-time retry planner with the policy's delay and attempt information. Under strict FIFO, `select_ready_item` sorts ready entries by enqueue sequence and item reference, then tests eligibility only for the earliest entry. It does not skip a delayed head to claim a later ready item. This head-of-line behavior follows from the selected profile; it is not a global ordering guarantee over worker completion or external effects.

## Worked lease-and-retry timeline

Consider an illustrative queue with item `A` before item `B`, using the reviewed profile. Consumer `worker-a` claims `A` at logical tick 100, so its visibility deadline is 110. At tick 104 it nacks with the supported transient class. Assuming all admission and capacity checks pass, `A` becomes retry-eligible at 109 and retains its original enqueue sequence.

At tick 108, a claim cannot bypass `A` to take `B` under strict FIFO. At tick 109, `A` can be claimed again with attempt two and a new fencing token. A delayed ack for the first claim does not complete this second claim: the active token differs. If attempt two expires, an admitted expiry request can dead-letter the item because the attempt budget is exhausted.

Now change only one detail: the worker performed an external write before its first nack. The state machine still follows the same retry rules. It has no basis to conclude that the write did not occur, and retry can cause the worker to encounter that external effect again. Delivery's at-least-once contract does not become exactly-once through token fencing.

## Review guidance and evidence limits

Suggested boundary checks include ack immediately before the deadline, ack at the deadline, expiry before and at the deadline, extension followed by an old-token ack, wrong-owner completion, and a delayed strict-FIFO head. Review denial against the original state, rather than inspecting only intermediate helper mutations. These are proposed verification cases, not executed evidence here.

Operation replay is separately identified: the same operation identifier and request reference can return duplicate replay without a new transition; reusing an operation identifier for different input is denied. This prevents treating request duplication as new state-machine work. It does not prove payload correctness or uniqueness of worker effects.

A claim supplies delivery facts, not worker execution authority. Content, provenance, policy, resources, execution admission, and evidence remain separate obligations. Canonical receipts document outcomes; they do not authorize future mutation or establish release eligibility.

## Sources

- [Coordination delivery extension](../../coordination-delivery.md)
- [Addressable actor delivery boundary](../../addressable-actor-runtime.md)
- [Delivery transition coordinator](../../../crates/molten-core/src/coordination_delivery/transition.rs)
- [Queue claim mechanics](../../../crates/molten-core/src/coordination_delivery/transition/queue.rs)
- [Completion and expiry mechanics](../../../crates/molten-core/src/coordination_delivery/transition/completion.rs)
- [Token checks, FIFO selection, and retry planning](../../../crates/molten-core/src/coordination_delivery/transition/support.rs)
- [Reviewed delivery profile](../../../config/coordination-delivery/profile.ncl)
- [Technical companion](../README.md)
