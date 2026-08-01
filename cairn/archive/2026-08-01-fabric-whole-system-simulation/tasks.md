## Phase 1: World and composition contracts

- [x] [serial] Add canonical simulated-world, workload, fault-plan, invariant, exploration, bound, and claim-profile artifacts. r[molten.fabric_simulation.world_manifest]
- [x] [serial] Build a simulation composition root that loads ordinary system-extension manifests, dispatchers, extension cores, schemas, and canonical port command/event types. r[molten.fabric_simulation.same_core]
- [x] [parallel] Add positive world fixtures and negative missing-port, ambient-input, duplicate-node, stale-generation, incompatible-profile, unbounded-run, and claim-overreach fixtures. r[molten.fabric_simulation.world_manifest] r[molten.fabric_simulation.same_core]

## Phase 2: Deterministic fabric adapters

- [x] [serial] Integrate simulated transport, durable-state, time, entropy, membership, placement, consistency, process-lifecycle, and resource adapters under one deterministic scheduler. r[molten.fabric_simulation.port_substitution] r[molten.fabric_simulation.scheduler]
- [x] [parallel] Add named network, disk, clock, scheduler, process, membership, resource, authority, and consistency fault actions without direct extension-state mutation. r[molten.fabric_simulation.fault_model]
- [x] [parallel] Add shared live/sim adapter conformance and differential traces for overlapping declared semantics. r[molten.fabric_simulation.live_sim_differential]

## Phase 3: Invariants, replay, and shrinking

- [x] [serial] Add pure extension-owned state/history invariants and universal fabric invariants over bounded redacted canonical observations. r[molten.fabric_simulation.invariants]
- [x] [serial] Add deterministic replay and first-divergence diagnostics over scheduler choices, entropy positions, faults, port events, lifecycle transitions, outputs, and state refs. r[molten.fabric_simulation.replay_shrink]
- [x] [parallel] Add causal shrinkers for workloads, faults, schedules, delays, resources, and eligible node sets plus minimal reproducibility bundles. r[molten.fabric_simulation.replay_shrink]

## Phase 4: Reference-system vertical slices

- [x] [serial] Implement a minimal transactional ordered key-value system extension with extension-owned transaction, conflict, commit, recovery, and invariant semantics. r[molten.fabric_simulation.reference_services]
- [x] [parallel] Implement a minimal replicated append-log system extension with extension-owned offsets, retention, replication, recovery, and history invariants. r[molten.fabric_simulation.reference_services]
- [x] [parallel] Implement a minimal distributed-scheduler system extension with extension-owned jobs, leases, retries, completion, failover, and no-double-authoritative-completion invariant. r[molten.fabric_simulation.reference_services]
- [x] [serial] Prove all three slices activate and run through fabric ports without node-core modifications, ambient authority, or mock-only service logic. r[molten.fabric_simulation.fabric_sufficiency]

## Phase 5: Evidence and profile ladder

- [x] [serial] Add pure-model, deterministic-simulation, multi-process-live, host-chaos, and VM/hardware claim profiles with fail-closed promotion rules. r[molten.fabric_simulation.claim_ladder]
- [x] [parallel] Emit bounded world, run, invariant, divergence, coverage, differential, shrink, and profile evidence compatible with sealed repro and cluster run-directory conventions. r[molten.fabric_simulation.evidence]
- [x] [parallel] Add CLI run, replay, shrink, inspect, and export flows with bounded status and no secret payload exposure. r[molten.fabric_simulation.operator_workflow]

## Phase 6: Validation

- [x] [serial] Run deterministic repeatability, fault matrices, invariant failures, replay divergence, shrinking, adapter differential, reference-service, ambient-I/O denial, cleanup, evidence, and profile-promotion tests. r[molten.fabric_simulation.final_validation]
- [x] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.fabric_simulation.final_validation]

## Resolved prerequisites

All seven declared prerequisites are archived. The last prerequisite,
`fabric-consistency-service-runtime`, was archived on 2026-07-24 after the
bounded cross-process transport shell was accepted. The simulation keeps live
consistency and deterministic substitution as separate claim profiles.
