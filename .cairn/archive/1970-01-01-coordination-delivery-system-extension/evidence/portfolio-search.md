# Coordination delivery portfolio search

## Goal

Select the weakest mechanism that adds fenced, durable, at-least-once work
delivery without changing the accepted base FIFO queue.

## Candidate A: Extend the base FIFO state machine

This would add claims, deadlines, acknowledgements, retries, and DLQs to the
existing enqueue/dequeue schema. It was rejected because accepted FIFO callers
would silently receive new timing, retention, replay, and failure semantics.

## Candidate B: Select an external broker as the delivery authority

Iggy, cloud queues, and similar brokers provide useful implementation and test
references. Direct selection was rejected because broker identity, clocks,
acknowledgements, and storage roots cannot become Molten policy, authority, or
semantic identity. The change must also remain testable without a live broker.

## Candidate C: Reuse Animus background-work queues

Animus exposes a narrow host-owned process queue contract. It does not own
visibility leases, consistency epochs, DLQ policy, logical-time expiry, or
Molten coordination authority. Its contract is compatible as a future worker
adapter, but it is not the delivery state machine.

## Candidate D: Compose accepted Molten primitives

The selected mechanism adds one Molten-local pure delivery core and a thin
imperative shell. It reuses:

- accepted coordination operation identity and duplicate handling;
- fabric logical deadlines, retry planning, and timer intents;
- live consistency currentness and engine-epoch evidence;
- durable-state compare-and-commit and restart boundaries;
- system-extension manifest and lifecycle ownership;
- whole-system simulation scheduling and fault inputs.

The core owns delivery policy and deterministic state transitions. The shell
owns storage, transactions, timer scheduling, status publication, restart, and
multiprocess effects. Payload execution remains behind separate content,
provenance, authority, policy, resource, and execution admission.

## Adversarial audit

The selected design fails closed on local-stale currentness, wall-clock expiry,
stale generations, stale epochs, wrong owners, token drift, conflicting
operation IDs, unknown commit outcomes, unsupported failures, exhausted
attempts, full DLQs, missing redrive authority, and incomplete worker admission.

Receipts describe one bounded transition and observed shell outcome. They do
not prove exact-once effects, payload correctness, global ordering, broker
correctness, authority, or release eligibility.
