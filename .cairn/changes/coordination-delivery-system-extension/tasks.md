## Phase 1: Versioned delivery model

- [ ] [serial] Define a coordination-delivery extension manifest and canonical item, claim, delivery token, attempt, ack, nack, expiry, retry, DLQ, redrive, state, status, and receipt records without changing base FIFO schemas. r[molten.coordination_delivery.versioned_extension] r[molten.coordination_delivery.claim_lease]
- [ ] [serial] Implement pure claim, ack, nack, lease extension, expiry, retry, dead-letter, redrive, duplicate-replay, and conflict-denial transitions. r[molten.coordination_delivery.claim_lease] r[molten.coordination_delivery.fenced_completion] r[molten.coordination_delivery.retry_dlq_policy]
- [ ] [parallel] Add positive and negative transition tests for owner, token, generation, epoch, logical deadline, attempts, duplicate operations, ordering posture, DLQ capacity, and redrive authority. r[molten.coordination_delivery.logical_time] r[molten.coordination_delivery.fenced_completion] r[molten.coordination_delivery.retry_dlq_policy]

## Phase 2: Consistency, durability, and time shell

- [ ] [serial] Host the delivery state machine as a system extension over admitted consistency, durable-state, time, resource, and observability ports. r[molten.coordination_delivery.consistency_durability]
- [ ] [serial] Bind claims and completions to normalized currentness, durability outcome, active engine epoch, delivery fencing token, service generation, and operation id. r[molten.coordination_delivery.fenced_completion] r[molten.coordination_delivery.consistency_durability]
- [ ] [parallel] Schedule visibility and retry eligibility only through admitted logical-time events and reject wall-clock or local-stale mutation paths. r[molten.coordination_delivery.logical_time]
- [ ] [parallel] Keep large payloads outside coordination state and require separate content, authority, provenance, resource, policy, and execution admission before worker effects. r[molten.coordination_delivery.content_refs]

## Phase 3: Recovery, simulation, and operator UX

- [ ] [parallel] Add crash-after-claim, restart, partition, expiry race, duplicate delivery, stale ack, retry, DLQ, redrive, overload, and cleanup scenarios to whole-system simulation. r[molten.coordination_delivery.final_validation]
- [ ] [parallel] Add local multiprocess delivery fixtures proving durable claim recovery and stale-consumer fencing. r[molten.coordination_delivery.consistency_durability] r[molten.coordination_delivery.final_validation]
- [ ] [parallel] Add bounded operator readback for ready/in-flight/retry/DLQ counts, active claims, attempt policy, logical deadlines, failures, resources, and evidence refs without payload rendering. r[molten.coordination_delivery.retry_dlq_policy]

## Phase 4: Validation

- [ ] [serial] Run focused properties and positive/negative delivery, consistency, durability, time, restart, simulation, multiprocess, authority, and exact-once non-claim tests. r[molten.coordination_delivery.final_validation]
- [ ] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.coordination_delivery.final_validation]

## Blocker

This package directly depends on `fabric-consistency-service-runtime` and on
`fabric-whole-system-simulation`; both are blocked by the missing admitted
cross-process Iroh listener/session shell. Claims, acknowledgements, expiry, and
redrive require real currentness and stale-consumer fencing evidence, so the
in-process control-registry model cannot discharge the dependency. Resume after
both dependencies are completed.
