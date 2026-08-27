## Phase 1: Replication manifest and pure planner

- [x] [serial] Define canonical replication manifests, replica policy, placement epochs, inventories, plans, operations, status assertions, and evidence with explicit system-extension authority. r[molten.content_replication.manifest]
- [x] [serial] Implement pure deterministic inventory diff, target selection, repair, handoff, deferral, pin, cleanup, idempotency, and epoch-fencing plans. r[molten.content_replication.planner] r[molten.content_replication.epoch_fencing]
- [x] [parallel] Add generated positive and negative planner tests for placement constraints, stable ordering, insufficient peers, stale epochs, conflicting operations, retention, and resource bounds. r[molten.content_replication.planner] r[molten.content_replication.epoch_fencing] r[molten.content_replication.resources_failures]

## Phase 2: Executable system extension

- [ ] [serial] Implement the supervised replication service lifecycle and bind content, transport, durable-state, time, membership, placement, identity, resource, and observability ports. r[molten.content_replication.manifest] r[molten.content_replication.same_core]
- [ ] [serial] Execute receiver-driven idempotent transfers, verification, replica-state updates, repair, handoff, cancellation, and restart through typed effects only. r[molten.content_replication.receiver_driven] r[molten.content_replication.resources_failures]
- [ ] [parallel] Integrate canonical retention pins and protected-content rules before transfer, source cleanup, unpin, or repair exposure. r[molten.content_replication.retention_confidentiality]

## Phase 3: Simulation, live evidence, and operations

- [ ] [parallel] Run the same extension core against deterministic content/transport/time/disk adapters with partition, corruption, cancellation, crash, placement-change, and resource-pressure faults. r[molten.content_replication.same_core] r[molten.content_replication.resources_failures]
- [ ] [parallel] Add live-loopback and local multiprocess replication, repair, restart, and stale-epoch fixtures with bounded aggregate evidence. r[molten.content_replication.same_core] r[molten.content_replication.final_validation]
- [ ] [parallel] Add operator readback for desired and verified replicas, placement epoch, under-replication, active plans, transfer resources, failures, pins, and non-claims. r[molten.content_replication.resources_failures]

## Phase 4: Validation

- [ ] [serial] Run placement/convergence properties and all positive and negative extension, adapter, retention, failure, restart, live/simulation, and authority tests. r[molten.content_replication.final_validation]
- [ ] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.content_replication.final_validation]

## Dependency resolution

`fabric-whole-system-simulation` is archived at `.cairn/archive/2026-08-01-fabric-whole-system-simulation`. All other declared fabric dependencies are also archived. Implementation can resume against the accepted same-core composition without extension-private mocks or relabeled loopback evidence.
