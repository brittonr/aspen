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

## Reference services

The exit fixture runs three system extensions.

### Transactional ordered key-value

The extension owns commit versions, conflicts, recovery, and state transitions.
A conflicting expected version returns an explicit conflict without mutation.

### Replicated append log

The extension owns offsets, replication progress, retention, and recovery.
Retention cannot move past the replicated boundary.

### Distributed scheduler

The extension owns jobs, leases, failover, and authoritative completion.
A wrong lease owner or second authoritative completion fails closed.

Every request enters `SystemExtensionHost` through the ordinary request callback.
Every approved effect routes through all admitted deterministic fabric port bindings.
No node-core branch matches a database, log, or scheduler operation.

These services do not claim FoundationDB, Kafka, or external scheduler compatibility.

## Differential scope

The fixture compares simulation and reviewed live descriptors at the shared command and event contract.
It does not run live networking or live storage.

The differential receipt records distinct simulation and live profile refs.
It also records the explicit no-live-equivalence check.

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
