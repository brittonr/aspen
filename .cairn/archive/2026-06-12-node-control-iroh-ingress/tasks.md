# Tasks: Node Control Iroh Ingress

## Phase 1: Canonical ingress artifacts

- [x] [serial] r[molten.node_control_ingress.spec.canonical_envelope] Add canonical ingress envelope and receipt schemas for local-Iroh node control ingress.
- [x] [serial] r[molten.node_control_ingress.spec.cli] Add CLI commands for building, publishing, and delivering local-Iroh ingress envelopes.

## Phase 2: Pre-enqueue control gates

- [x] [serial] r[molten.node_control_ingress.spec.pre_enqueue_gates] Validate peer bootstrap, authority, policy, resource, and delivery idempotency before enqueue.
- [x] [serial] r[molten.node_control_ingress.spec.durable_inbox] Deliver admitted ingress envelopes into the existing durable control inbox without bypassing dispatch.
- [x] [serial] r[molten.node_control_ingress.spec.duplicate_replay] Suppress duplicate remote ingress operations and deny stale/gap/conflict deliveries before side effects.

## Phase 3: Coverage and validation

- [x] [parallel] r[molten.node_control_ingress.spec.tests] Cover pass, duplicate suppression, missing authority denial, and provenance-gated dispatch behavior.
- [x] [serial] r[molten.node_control_ingress.spec.tests] Run Molten validation gates and Cairn strict validation with the checked-out Cairn policy.
