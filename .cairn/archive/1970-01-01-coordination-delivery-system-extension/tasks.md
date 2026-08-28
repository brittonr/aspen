## Phase 1: Versioned delivery model

- [x] [serial] Define a coordination-delivery extension manifest and canonical item, claim, delivery token, attempt, ack, nack, expiry, retry, DLQ, redrive, state, status, and receipt records without changing base FIFO schemas. r[molten.coordination_delivery.versioned_extension] r[molten.coordination_delivery.claim_lease]
- [x] [serial] Implement pure claim, ack, nack, lease extension, expiry, retry, dead-letter, redrive, duplicate-replay, and conflict-denial transitions. r[molten.coordination_delivery.claim_lease] r[molten.coordination_delivery.fenced_completion] r[molten.coordination_delivery.retry_dlq_policy]
- [x] [parallel] Add positive and negative transition tests for owner, token, generation, epoch, logical deadline, attempts, duplicate operations, ordering posture, DLQ capacity, and redrive authority. r[molten.coordination_delivery.logical_time] r[molten.coordination_delivery.fenced_completion] r[molten.coordination_delivery.retry_dlq_policy]

## Phase 2: Consistency, durability, and time shell

- [x] [serial] Host the delivery state machine as a system extension over admitted consistency, durable-state, time, resource, and observability ports. r[molten.coordination_delivery.consistency_durability]
- [x] [serial] Bind claims and completions to normalized currentness, durability outcome, active engine epoch, delivery fencing token, service generation, and operation id. r[molten.coordination_delivery.fenced_completion] r[molten.coordination_delivery.consistency_durability]
- [x] [parallel] Schedule visibility and retry eligibility only through admitted logical-time events and reject wall-clock or local-stale mutation paths. r[molten.coordination_delivery.logical_time]
- [x] [parallel] Keep large payloads outside coordination state and require separate content, authority, provenance, resource, policy, and execution admission before worker effects. r[molten.coordination_delivery.content_refs]

## Phase 3: Recovery, simulation, and operator UX

- [x] [parallel] Add crash-after-claim, restart, partition, expiry race, duplicate delivery, stale ack, retry, DLQ, redrive, overload, and cleanup scenarios to whole-system simulation. r[molten.coordination_delivery.final_validation]
- [x] [parallel] Add local multiprocess delivery fixtures proving durable claim recovery and stale-consumer fencing. r[molten.coordination_delivery.consistency_durability] r[molten.coordination_delivery.final_validation]
- [x] [parallel] Add bounded operator readback for ready/in-flight/retry/DLQ counts, active claims, attempt policy, logical deadlines, failures, resources, and evidence refs without payload rendering. r[molten.coordination_delivery.retry_dlq_policy]

## Phase 4: Validation

- [x] [serial] Run focused properties and positive/negative delivery, consistency, durability, time, restart, simulation, multiprocess, authority, and exact-once non-claim tests. r[molten.coordination_delivery.final_validation]
- [x] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.coordination_delivery.final_validation]

## Dependency closure

All declared Molten dependencies are archived on canonical `origin/molten`.
The accepted cross-process Iroh listener/session shell, live consistency
service, durable-state ports, logical-time scheduler, observability integrity,
and same-core whole-system simulation now supply the required composition
boundaries. This change remains responsible for delivery policy, transitions,
receipts, and at-least-once non-claims.
