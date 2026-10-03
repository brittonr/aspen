# Deterministic simulation inputs

Determinism in Molten is a property of a closed, admitted experiment, not merely a consequence of choosing a pseudorandom seed. This article explains the input boundary of whole-system Fabric Simulation and the distinction between reproducible decisions and real-world execution. Read the [whole-system simulation guide](../../fabric-whole-system-simulation.md) first; the [Technical companion](../README.md) situates this discussion among the other implementation articles.

## Closing the experiment

A world binds nodes, active generations, extension identities, initial state, membership, placement, workload, scheduler and entropy inputs, faults, invariants, and finite execution bounds. The core receives these values rather than discovering them through the environment. The shell owns host composition, canonical Preserves artifacts, and command orchestration; the pure core performs no filesystem, network, process, clock, or environment access. This separation makes an input audit possible: an unexplained outcome should lead to an explicit input or a missing model assumption, not an implicit host dependency.

The distinction matters even for apparently harmless details. A host timestamp used to break ties is a behavioral input, not diagnostic decoration. A temporary directory may locate shell artifacts without becoming an admissible determinant of modeled scheduling. The [distributed testing guide](../../distributed-testing.md) similarly excludes host paths, process identifiers, real sockets, wall-clock time, and ambient randomness from simulation fault-plan inputs.

Admission is concrete rather than aspirational. `admit_simulated_world` collects identity, bounds, node, port, workload, fault, invariant, and claim issues before returning an admitted world. It rejects a nonempty `ambient_inputs` collection, validates required refs, and rejects zero bounds. Only after validation does it normalize collection order: nodes by identity, ports by class, workload by sequence, faults by identity, and invariants and non-claims by their defined ordering. See [admission.rs](../../../crates/molten-core/src/fabric_simulation/admission.rs). Normalization does not excuse ambiguous input; duplicate identities remain errors rather than being silently merged.

## Identity is a contract comparison

The same-core check compares seven declared references between simulation and reviewed live descriptors: implementation, manifest, callback dispatcher, protocol core, state machine, schema set, and port-contract set. This is explicit reference equality. It is not Rust-layout equality, an assertion about memory representation, or proof that shell implementations behave identically.

The distinction between a well-formed reference and available evidence is equally important. The admission helper checks the `blake3:` prefix and hexadecimal shape; its same-core comparison checks supplied values. These functions do not independently establish that every referenced artifact was executed in a live environment. The governing guide consequently describes differential comparison at the shared command/event contract while retaining an explicit no-live-equivalence boundary.

## Scheduling needs more than a seed

The [scheduler implementation](../../../crates/molten-core/src/fabric_simulation/scheduler.rs) validates each eligible choice against the admitted node and generation. It rejects duplicate choice identifiers, stale generations, unknown nodes, an empty eligible set, and attempts to continue a terminal scheduler. Checked increments enforce choice and event bounds, while virtual time advances to at least the selected choice's readiness tick and remains within the world bound.

`select_simulation_choice` sorts eligible alternatives. Without a recorded identifier it selects the first ordered alternative; with a recorded identifier it requires that exact alternative to be present. A separate `seeded_selection_index` helper supplies deterministic seed/position indexing. Thus “seeded simulation” should not be paraphrased as “every scheduler call automatically samples the seed.” Review the actual selection path together with its input refs.

Replay also compares more than final state. `compare_replay` checks ordered records for position, virtual tick, selected generation, choice identifier, semantic output reference, and eligible-choice identifiers, then checks trace length. The [replay implementation](../../../crates/molten-core/src/fabric_simulation/replay.rs) reports the first divergence rather than treating eventual convergence as equivalent execution.

## Worked input-drift example

Consider an illustrative world with node `worker-a` at generation 4 and two eligible deliveries. A recorded choice addresses generation 4. A later experiment retains the seed and workload but starts `worker-a` at generation 5. Those are different experiments: the old delivery is stale against the new manifest, even if both runs would eventually produce the same service state.

A second illustrative change merely permutes the manifest's node list. Admission sorts that collection after validating uniqueness, so collection presentation alone need not alter normalized order. By contrast, changing workload sequence numbers changes semantic ordering and cannot be dismissed as presentation. Together these examples show why reviewers need the admitted world and trace, not a seed printed in a log.

## Review and verification guidance

Suggested verification, not executed evidence in this article, is to use the guide's `fabric-simulation preflight`, `run`, `inspect`, and `replay` commands. Inspect world and run refs alongside ordered choices. For a negative review case, remove a required port, introduce an ambient input, or make a replayed choice unavailable; the meaningful result is explicit rejection, not an automatically repaired run.

Check that finite bounds, same-core descriptors, and required non-claims survive artifact export. A passing bounded simulation does not establish live disk behavior, operating-system timing, transport execution, production scale, production readiness, or correctness for unexplored schedules. Those are separate evidence obligations, not extrapolations from deterministic replay.

## Sources

- [Whole-system Fabric Simulation](../../fabric-whole-system-simulation.md)
- [Distributed testing evidence](../../distributed-testing.md)
- [World admission and identity checks](../../../crates/molten-core/src/fabric_simulation/admission.rs)
- [Scheduler transitions](../../../crates/molten-core/src/fabric_simulation/scheduler.rs)
- [Replay comparison and shrinking](../../../crates/molten-core/src/fabric_simulation/replay.rs)
- [Technical companion](../README.md)
