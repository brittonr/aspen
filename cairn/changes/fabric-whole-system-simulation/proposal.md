## Why

Aspen's harness already specifies deterministic artifacts, replay, adapter conformance, and multi-peer simulation, but system extensions do not yet have one whole-system simulation composition that substitutes transport, durable state, time, entropy, membership, placement, consistency, process lifecycle, and resources while running the same extension protocol and state-transition core used in live nodes.

Without that composition, each distributed service can drift into a separate mock implementation, and passing tests cannot demonstrate that the live extension path was exercised.

## What Changes

- Add a canonical simulated-world manifest for nodes, system extensions, port profiles, initial durable state, membership, resources, scheduler choices, entropy, workloads, faults, invariants, and bounds.
- Compose the ordinary system-extension dispatcher and extension cores with deterministic fabric-port adapters rather than mock-only callbacks or private state mutation.
- Model network, disk, clock, scheduler, process, membership, resource, and consistency faults at named canonical boundaries.
- Add deterministic replay, first divergence, state/history hashing, counterexample shrinking, and minimal repro bundles for whole-system runs.
- Add differential conformance between simulated and live adapter profiles where the declared semantics overlap.
- Validate fabric sufficiency with minimal transactional key-value, replicated-log, and distributed-scheduler extension slices, without claiming FoundationDB, Kafka, or external scheduler compatibility.
- Define a claim ladder separating pure model, deterministic simulation, multi-process live, host-chaos, and VM evidence.

## Impact

- **Files**: simulation world models, composition root, simulated port adapters, deterministic scheduler/fault engine, invariant API, workload drivers, reference extensions, replay/shrink artifacts, CLI/operator readback, and a new `fabric-simulation` accepted spec.
- **Testing**: same-core identity, port substitution, deterministic replay, fault matrices, state/history invariants, shrink stability, reference services, simulated/live differential traces, ambient-I/O denial, and evidence-scope tests.
- **Safety**: simulation evidence is scoped to declared models and schedules; it does not prove live transport, disk, OS, timing, scale, production readiness, or external product compatibility.
