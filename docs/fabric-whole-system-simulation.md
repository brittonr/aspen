# Whole-system fabric simulation

Molten provides one bounded, deterministic composition for system extensions.

The composition runs ordinary system-extension manifests, callbacks, state-machine code, and typed fabric effects.
It replaces shell adapters and the top-level scheduler only.

## Claim boundary

A passing run is deterministic whole-system simulation evidence.
It is not live or production evidence.

The required non-claims are:

- No proof of live transport
- No proof of live disk behavior
- No proof of operating-system timing
- No proof of production scale
- No proof of production readiness
- No external product compatibility claim
- No proof for schedules outside the explored bounds

A stronger profile needs its own implementation, environment, adapter, lifecycle, fault, and operator evidence.
The claim gate fails closed when that evidence is absent.

## Functional boundary

The pure core is in `crates/molten-core/src/fabric_simulation/`.
It performs no filesystem, network, process, clock, or environment access.

The pure core owns:

- World admission and normalization
- Same-core identity comparison
- Scheduler choices and replay checks
- Fault admission at named boundaries
- Invariant evaluation
- Claim-profile promotion decisions
- Causal shrinking
- Reference-service state transitions

The shell is in `src/fabric_simulation/`.
It owns canonical Preserves values, BLAKE3 refs, host composition, artifact output, and CLI orchestration.

## World closure

A world binds all behavior inputs before execution.
The manifest includes:

- Nodes and active generations
- Extension and service identities
- Implementation, manifest, dispatcher, protocol, state-machine, schema, and port-contract refs
- Initial state, membership, placement, and consistency refs
- All thirteen fabric port classes
- Workload steps and request refs
- Scheduler and entropy input refs
- Named faults and activation positions
- Universal and extension-owned invariants
- Choice, event, time, trace, resource, and shrink bounds
- Claim profile and required non-claims

Admission rejects missing ports, duplicate nodes, stale generations, unbounded values, ambient inputs, and stronger claim labels.
Admission also rejects direct fault mutation of extension state.

The review profile is `docs/fabric-whole-system-simulation/profile.ncl`.
Its typed contract is `docs/fabric-whole-system-simulation/contracts.ncl`.
Negative Nickel fixtures cover missing ports, ambient input, zero bounds, and claim overreach.

## Scheduler

One scheduler controls each modeled choice.
It controls runnable selection, delivery, timers, storage completion, process completion, and fault activation.

Each choice records:

- Canonical position
- Virtual time
- Eligible alternatives
- Selected alternative
- Node and generation

Replay stops at the first unavailable recorded choice.
It does not choose a replacement silently.

## Faults

Faults occur only at a declared port or lifecycle boundary.
The model supports:

- Delay, drop, duplicate, reorder, partition, and reset
- Bounded corruption and capacity exhaustion
- Pause, crash, and restart
- Clock skew and clock jump
- Authority revocation
- Membership change and placement replacement
- Consistency quorum loss

Each fault names its target, boundary, activation, duration, resource cost, and expected observation.
A fault cannot patch an extension transaction table, log, job map, or other semantic state.

## Invariants

The fabric checks six universal invariants:

1. No ambient effect
2. No stale-generation mutation
3. No resource-bound bypass
4. No port state-machine violation
5. Valid canonical refs
6. Complete terminal cleanup

Each reference extension owns its semantic invariants.
The fabric does not define transaction, log, or job semantics.

## Protocol-aware oracles

r[impl molten.fabric_simulation.oracle_independence] A transition's own report never decides an extension-semantic invariant. Observations carry no pass list. Each records the canonical ref of a protocol projection, and the independent oracle (`molten_core::protocol_oracle`) decides every result.

**Projections.** Every reference participant projects its initial state, each admitted transition, and each recovery from the durable image. A projection binds, among other facts:

- the participant, its generation, and its source and run sequences;
- the scheduler choice, the transition class, and the logical position;
- the canonical state and its ref, the history ref, and the durability fact;
- progress and observation completeness.

The transition's reported invariant names travel with the projection as a non-evidential field.

The extension's projector emits canonical Preserves values, and refs are domain-separated BLAKE3 over the canonical encoding. Executor state refs and final state refs use the same projector instead of Rust debug text. A projection whose state ref is not the canonical ref of its state is refused.

**Profile.** `config/protocol-oracle/profile.ncl` is the typed profile. It names the services and their logical positions, the safety levels, the participants, and the completeness mode. It also sets the liveness preconditions, novelty fields, work counters, bounds, and non-claims. Its generated JSON is admitted by `parse_protocol_oracle_profile`.

**Cohort.** A lossless cohort with a sequence gap, a missing required participant, or a partial observation is incomplete and cannot pass. Duplicate sequences and generation drift are malformed.

**Safety levels.** Each result keeps its level and its earliest failure, with the scheduler choice and projection refs:

- *Local:* the extension's rules over one participant's consecutive projections.
- *Pairwise and cohort:* agreement at shared logical positions. Physical refs are compared only under a canonical-physical profile.
- *Durability:* a recovered state must be one the participant held at the same position.

**Local guards.** The same pure function, `reference_local_violations`, serves the oracle and the executor guard. The executor applies it to every transition under a fail-stop policy. Malformed commands stay typed admission errors, never invariant violations.

**Liveness.** Liveness is participant-scoped: pass, fail, not evaluated when a precondition is missing (a crash fault makes every participant not evaluated), or incomplete when the end or the virtual-progress horizon was not observed.

**Novelty and counters.** Novelty identities are canonical refs over the profile's fields; `explore_reference_seeds` scores the new protocol state each seed reaches. Work counters are monotonic and compared only within one world, runtime, scheduler, workload, fault, schema, and bounds cohort.

**Evidence.** The run record binds a bounded protocol status, which `fabric-simulation inspect` prints. Evidence binds every identity, result, novelty ref, counter, and non-claim. Only a deterministic-simulation claim is admissible over it; KVM, live, production, release, and hardware-performance claims are refused.

`config/protocol-oracle/consumer-contract.ncl` publishes the projection record, encoding, and identity derivation for a later external adapter. It is checked against the core's field list.

An independent oracle can share a defect with the implementation it checks. Results hold only for the recorded world, schedule, workload, faults, and projections.

## Reference services

The exit fixture runs three system extensions.

### Transactional ordered key-value

The extension owns commit versions, conflicts, recovery, and state transitions.
A conflicting expected version returns an explicit conflict without mutation.

### Replicated append log

The extension owns offsets, replication progress, retention, and recovery.
Retention cannot move past the replicated boundary.

### Distributed scheduler

The extension owns jobs, leases, failover, authoritative completion, and assignment-token binding.
A lease or failover establishes a new assignment epoch.
A completion must carry the token derived from the job, the owner, and the epoch of the assignment that produced the work.
A stale token, a wrong lease owner, or a second authoritative completion fails closed.
A separate pure checker derives assignments from recorded history and detects an accepted stale completion without consulting reported invariant names.

Every request enters `system_extension::Dispatcher` through the ordinary request callback.
Every approved effect routes through all admitted deterministic fabric port bindings.
No node-core branch matches a database, log, or scheduler operation.

These services do not claim FoundationDB, Kafka, or external scheduler compatibility.

## Differential scope

The fixture produces two separately identified differential artifacts.

The descriptor-level structural differential compares the declared simulation and reviewed live port-contract descriptors at the shared command and event contract. It does not run networking or storage, and it cannot diverge by construction.

The executed-workload differential runs one bounded shared workload through the simulated adapter and, when a live adapter is selected for the profile, through that live adapter. Each step declares its permitted outcome classes and an optional ordering key. The comparison normalizes observed outcomes to those classes, evaluates per-key ordering, and reports exactly one of agreement, permitted divergence, incomplete, or failure. Physical timing, latency, and non-canonical physical bytes do not participate. A live outcome outside its declared permitted set is the only failure; an unsupported behavior or unavailable adapter is incomplete and never agreement.

Evidence binds both artifact identities, the shared workload, the execution result, the incomplete count, and the non-claims. This fixture selects no live adapter, so its execution result is incomplete rather than agreement; agreement is exercised through the focused canonical tests with explicit executor stubs.

The differential does not establish live equivalence beyond the exercised workload and declared properties, performance, durability, or release eligibility.

## Replay and shrinking

Replay recomputes the reference world and compares the ordered scheduler trace.
World and run refs must also match.

The shrinker removes only candidates that remain valid and preserve the selected failure.
It re-admits each candidate from the canonical initial world.
Invalid candidates are rejected without hidden repair.

The fixture can reduce workload suffixes, unused faults, eligible unused nodes, trace bounds, and resource bounds.

## Operator commands

Preflight the reference world:

```sh
cargo run -- fabric-simulation preflight
```

Run the three reference services:

```sh
cargo run -- fabric-simulation run --out target/fabric-simulation-run
```

Inspect the bounded report:

```sh
cargo run -- fabric-simulation inspect target/fabric-simulation-run/report.preserves
```

Replay the report:

```sh
cargo run -- fabric-simulation replay target/fabric-simulation-run/report.preserves
```

Run the causal shrink fixture:

```sh
cargo run -- fabric-simulation shrink --out target/fabric-simulation-shrink
```

Export the compact offline bundle:

```sh
cargo run -- fabric-simulation export --out target/fabric-simulation-export
```

The run directory contains the world, run report, repro bundle, differential report, observations, and port events.
The compact export contains only the four top-level canonical artifacts.

Reports contain refs, bounded counters, decisions, profiles, and redacted observations.
They do not contain private keys, bearer tokens, environment values, or secret bytes.
