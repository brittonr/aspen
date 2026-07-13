## Phase 1: World and composition contracts

- [ ] [serial] Add canonical simulated-world, workload, fault-plan, invariant, exploration, bound, and claim-profile artifacts. r[molten.fabric_simulation.world_manifest]
- [ ] [serial] Build a simulation composition root that loads ordinary system-extension manifests, dispatchers, extension cores, schemas, and canonical port command/event types. r[molten.fabric_simulation.same_core]
- [ ] [parallel] Add positive world fixtures and negative missing-port, ambient-input, duplicate-node, stale-generation, incompatible-profile, unbounded-run, and claim-overreach fixtures. r[molten.fabric_simulation.world_manifest] r[molten.fabric_simulation.same_core]

## Phase 2: Deterministic fabric adapters

- [ ] [serial] Integrate simulated transport, durable-state, time, entropy, membership, placement, consistency, process-lifecycle, and resource adapters under one deterministic scheduler. r[molten.fabric_simulation.port_substitution] r[molten.fabric_simulation.scheduler]
- [ ] [parallel] Add named network, disk, clock, scheduler, process, membership, resource, authority, and consistency fault actions without direct extension-state mutation. r[molten.fabric_simulation.fault_model]
- [ ] [parallel] Add shared live/sim adapter conformance and differential traces for overlapping declared semantics. r[molten.fabric_simulation.live_sim_differential]

## Phase 3: Invariants, replay, and shrinking

- [ ] [serial] Add pure extension-owned state/history invariants and universal fabric invariants over bounded redacted canonical observations. r[molten.fabric_simulation.invariants]
- [ ] [serial] Add deterministic replay and first-divergence diagnostics over scheduler choices, entropy positions, faults, port events, lifecycle transitions, outputs, and state refs. r[molten.fabric_simulation.replay_shrink]
- [ ] [parallel] Add causal shrinkers for workloads, faults, schedules, delays, resources, and eligible node sets plus minimal reproducibility bundles. r[molten.fabric_simulation.replay_shrink]

## Phase 4: Reference-system vertical slices

- [ ] [serial] Implement a minimal transactional ordered key-value system extension with extension-owned transaction, conflict, commit, recovery, and invariant semantics. r[molten.fabric_simulation.reference_services]
- [ ] [parallel] Implement a minimal replicated append-log system extension with extension-owned offsets, retention, replication, recovery, and history invariants. r[molten.fabric_simulation.reference_services]
- [ ] [parallel] Implement a minimal distributed-scheduler system extension with extension-owned jobs, leases, retries, completion, failover, and no-double-authoritative-completion invariant. r[molten.fabric_simulation.reference_services]
- [ ] [serial] Prove all three slices activate and run through fabric ports without node-core modifications, ambient authority, or mock-only service logic. r[molten.fabric_simulation.fabric_sufficiency]

## Phase 5: Evidence and profile ladder

- [ ] [serial] Add pure-model, deterministic-simulation, multi-process-live, host-chaos, and VM/hardware claim profiles with fail-closed promotion rules. r[molten.fabric_simulation.claim_ladder]
- [ ] [parallel] Emit bounded world, run, invariant, divergence, coverage, differential, shrink, and profile evidence compatible with sealed repro and cluster run-directory conventions. r[molten.fabric_simulation.evidence]
- [ ] [parallel] Add CLI run, replay, shrink, inspect, and export flows with bounded status and no secret payload exposure. r[molten.fabric_simulation.operator_workflow]

## Phase 6: Validation

- [ ] [serial] Run deterministic repeatability, fault matrices, invariant failures, replay divergence, shrinking, adapter differential, reference-service, ambient-I/O denial, cleanup, evidence, and profile-promotion tests. r[molten.fabric_simulation.final_validation]
- [ ] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.fabric_simulation.final_validation]

## Blocker

This package depends on `fabric-consistency-service-runtime`, which is blocked
until the transport port exposes a bounded admitted cross-process Iroh listener
and session shell. The world manifest requires a consistency profile and the
reference slices require honest consistency fault substitution; substituting the
in-process control-registry model as live consistency would violate the claim
ladder. Resume after that dependency is unblocked.
