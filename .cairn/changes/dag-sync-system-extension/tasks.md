## Phase 1: Generic DAG primitives

- [x] [serial] Define canonical domain-neutral DAG node, edge, root, bounds, inventory, request, plan, response, progress, and receipt records. r[molten.dag_sync.model]
- [x] [serial] Implement pure cycle and duplicate detection, edge validation, deterministic traversal/topological order, visited-state, missing-set, completion, and hard-bound checks. r[molten.dag_sync.traversal_core]
- [x] [parallel] Add property and negative tests for stable order, cycles, unknown edges, duplicate nodes, node/edge/depth/byte/step limits, and malformed records. r[molten.dag_sync.traversal_core]

## Phase 2: Strategies and extension protocol

- [x] [serial] Add explicit full, stem-first, leaf-only, resumable, and deterministic peer-partitioned strategy profiles with canonical plan identity. r[molten.dag_sync.strategy_profiles]
- [x] [serial] Implement the receiver-driven DAG-sync system extension over admitted transport, content, durable-state, time, identity, resource, and observability ports. r[molten.dag_sync.receiver_driven] r[molten.dag_sync.content_adapter_boundary]
- [x] [parallel] Reject unsolicited, stale, duplicate-conflicting, corrupt, unauthorized, or over-bound responses before graph progress advances. r[molten.dag_sync.receiver_driven] r[molten.dag_sync.content_adapter_boundary]

## Phase 3: Resume and domain integrations

- [x] [serial] Persist bounded verified progress and implement traversal-epoch, root, schema, strategy, peer-assignment, policy, and generation checks for safe resume. r[molten.dag_sync.resume_fencing]
- [x] [parallel] Integrate job DAG and artifact dependency-closure callers through generic roots and completion evidence without granting install, execution, publication, or merge authority. r[molten.dag_sync.domain_boundary]
- [ ] [parallel] Add same-core deterministic simulation and live-loopback fixtures for complete, partial, restart, cancellation, partition, peer reassignment, and corruption paths. r[molten.dag_sync.final_validation]
- [x] [parallel] Add bounded operator readback for roots, strategy, traversal epoch, requested/verified/missing refs, peers, progress, resources, failures, and evidence refs. r[molten.dag_sync.resume_fencing]

## Phase 4: Validation

- [ ] [serial] Run traversal properties and all positive/negative protocol, content, resume, simulation/live parity, authority, and domain-boundary tests. r[molten.dag_sync.final_validation]
- [ ] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.dag_sync.final_validation]

## Dependency resolution

`fabric-whole-system-simulation` is archived at `.cairn/archive/2026-08-01-fabric-whole-system-simulation`. All other declared fabric dependencies are also archived. Implementation can resume against the accepted same-core composition without a DAG-specific mock world.
