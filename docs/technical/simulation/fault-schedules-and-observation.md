# Fault schedules and observation

A fault schedule is an explicit intervention at a modeled boundary, not permission to edit a service into a desired failure state. This article examines admission, stateful transport effects, observations, and causal reduction in Fabric Simulation. It assumes familiarity with the [whole-system simulation guide](../../fabric-whole-system-simulation.md); see the [Technical companion](../README.md) for adjacent topics.

## Intervention and semantic ownership

Fabric faults name a target, boundary, activation, duration, resource cost, and expected observation. The governing model includes transport disturbances, resource pressure, process lifecycle changes, time disturbances, authority revocation, membership changes, placement replacement, and quorum loss. That vocabulary describes modeled interventions. It does not imply that a given platform adapter implements every fault, nor that all combinations have been explored.

The admission path verifies that each fault target names an admitted node or port, that its boundary has an admitted profile, and that the profile declares the fault kind. It rejects duplicate fault identities, zero resource cost, and `direct_extension_state_mutation`. These checks appear in [admission.rs](../../../crates/molten-core/src/fabric_simulation/admission.rs). A fault that directly rewrites an extension's transaction table would bypass the very transition logic the experiment is meant to exercise.

This preserves a useful ownership boundary. The fabric checks universal invariants such as stale-generation mutation and resource-bound bypass; extensions own transaction conflicts, replicated-log retention, or authoritative job completion. Injecting a delivery interruption may expose an extension error. Defining the extension's correct recovery decision inside the injector would instead collapse implementation and oracle into one component.

## Stateful transport, not renamed events

The [deterministic adapters](../../../crates/molten-core/src/fabric_simulation/adapters.rs) maintain separate pending, dropped, delivered, active-partition, and healed-partition collections. A submitted transmission carries destination, port, request reference, generation, submission tick, and eligibility tick. Delaying a pending transmission raises its eligibility tick using a maximum; it does not move the transmission backward in modeled time. Dropping removes the pending entry and records it in the dropped collection.

A partition targets a destination and has a healing tick. The transport computes effective readiness using the maximum of transmission eligibility and applicable partition healing ticks. Eligibility for delivery additionally requires that the destination is not currently blocked. `heal_ready_partitions` moves partitions whose healing time has arrived out of the active collection. Consequently, advancing the clock and applying the corresponding lifecycle transition are distinct operations in this state model; reading a future healing timestamp is not itself a mutation of partition state.

The scheduler then operates over eligible alternatives. Its validation checks node identity and generation; it does not authorize an old-generation delivery merely because the fault plan mentions it. A recorded choice that is no longer eligible becomes an explicit replay divergence, preserving the causal significance of the boundary.

## Worked partition-and-delay scenario

Suppose, illustratively, transmission `t1` targets `node-b`, becomes eligible at tick 5, and is delayed until tick 9. A partition blocks `node-b` until tick 12. The delay alone would permit delivery at 9; the partition means the earliest modeled readiness is 12. Before healing the active partition, `eligible_deliveries` still excludes `t1`, even at tick 12. After the partition transition is applied, delivery may become eligible.

Now suppose a replay requests the delivery at the earlier position occupied by a timer in the recorded experiment. Selecting some other runnable event would hide the mismatch. Instead, the scheduler rejects an unavailable recorded choice. The diagnostic question becomes precise: did the partition activation differ, was healing omitted, or did a changed prior event alter the eligible set?

This example is reasoning over inspected adapter functions, not a reported run. It demonstrates why fault presence, activation, observed state transition, and scheduler trace should be inspected separately. A text line saying “partition injected” does not establish which message was prevented from progressing.

## Observation and causal shrinking

Invariant evaluation scans observations for the first failure of each declared invariant. Extension-semantic checks are scoped to the observation's service; universal checks inspect their corresponding observation fields. This makes the observation producer part of the evidence boundary: a passing evaluator is agreement with supplied observations, not an independent sensor for unmodeled shell activity.

The [shrinker](../../../crates/molten-core/src/fabric_simulation/replay.rs) first admits and reruns the original world. Its failure fingerprint binds decision, failed-invariant identities, and first-failure sequence. Candidate reductions remove workload suffix steps, trailing faults, eligible unused nodes, or reduce positive resource and trace bounds. Each candidate is re-admitted and retained only when rerunning preserves the fingerprint. Invalid candidates are rejected without hidden repair.

This is bounded causal reduction, not a claim of a globally smallest counterexample. Preserving the first-failure sequence is also stricter than preserving a broad English description such as “a transaction failed.” A reduced artifact should therefore be read with its exact failure criterion.

## Review guidance and limits

Suggested verification is to inspect a run's admitted fault plan, scheduler choices, observations, and port events together, then use the documented shrink command on its supported fixture. Review denial paths for direct semantic mutation, undeclared boundary faults, stale-generation alternatives, and unavailable replay choices. These are proposed checks, not executed results here.

The [distributed testing guide](../../distributed-testing.md) distinguishes diagnostic generated repro bundles from separately accepted pass/deny claims. Neither such a bundle nor a bounded simulation receipt grants authority, deployment trust, exactly-once effects, or live network evidence. Unexplored schedules remain outside the result, even when the named fault categories appear comprehensive.

## Sources

- [Whole-system Fabric Simulation](../../fabric-whole-system-simulation.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [Fault admission and invariant evaluation](../../../crates/molten-core/src/fabric_simulation/admission.rs)
- [Stateful deterministic adapters](../../../crates/molten-core/src/fabric_simulation/adapters.rs)
- [Scheduler choice validation](../../../crates/molten-core/src/fabric_simulation/scheduler.rs)
- [Failure fingerprints and causal shrinking](../../../crates/molten-core/src/fabric_simulation/replay.rs)
- [Technical companion](../README.md)
