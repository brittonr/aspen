# Design: Bounded execution fabric port

## Context

Molten's system-extension host exposes typed effects and exact fabric-port bindings. The port registry has no native execution class.

Some extensions need reviewed external tools. Direct `std::process::Command`, inherited environment, ambient current directories, or host paths would bypass authority, resource, replay, and simulation boundaries.

The `bounded-exec` repository provides the required process mechanics. Its reviewed pilot revision is `29dac88ecded94457572db3fdfaaaab95fa91525`.

## Success Contract

Completion requires one application-owned port, one pure admission core, one live adapter, and one deterministic simulation adapter.

A valid request must bind an authorized executable artifact, explicit arguments, cleared environment, input, workspace, limits, generation, authority, resources, and idempotency identity.

False completion includes a direct process helper, adapter-owned policy, raw paths in canonical authority, inherited environment, unbounded output, hidden retry, or application success inferred from exit status.

## Decisions

### Decision: Add a distinct execution port class

**Choice:** Add `Execution` as a fabric-port class with a versioned `molten.fabric.execution.v1` descriptor family.

The port requires explicit process-execution authority and named memory, concurrency, queue, storage, logical-time, and diagnostic resource bindings.

**Rationale:** Process execution is not transport, scheduling, supervision, or application workload meaning.

### Decision: Reuse Bounded Exec unchanged

**Choice:** Pin the published Bounded Exec component and adapt its request and observation types in the live shell.

Molten does not copy the component source. The adapter records the exact source revision, license, platform support, and claim boundary.

**Rationale:** The component already owns the required product-neutral process mechanics.

### Decision: Applications own executable policy

**Choice:** The consuming application supplies executable artifact, provenance, authority, effect-manifest, workspace, environment, and outcome-policy facts.

The pure core validates these supplied facts against the selected execution profile. The adapter does not discover executables or choose policy.

**Rationale:** Mechanism availability does not grant execution authority.

### Decision: Canonical requests contain logical references

**Choice:** Canonical requests use content, artifact, workspace, input, policy, authority, resource, and profile references.

The shell resolves those references to already-open capability handles. Diagnostic host paths and process identifiers stay outside canonical receipts.

**Rationale:** Ambient paths are neither portable identity nor authority.

### Decision: Production starts with a cleared environment

**Choice:** The first production profile requires an empty inherited environment and an explicit bounded environment map.

Unsupported inheritance, duplicate keys, secret bytes, path search, shell expansion, and implicit current directories deny before spawn.

**Rationale:** Inherited process state creates hidden inputs and authority.

### Decision: Lifecycle outcomes remain explicit

**Choice:** Model admitted, queued, started, exited, timed-out, cancelled, failed-before-start, failed-after-start, teardown-incomplete, and unknown states.

Exit codes and signals are observations. The consumer maps them to application outcomes after its own policy runs.

A host failure after start and before definitive teardown or completion remains unknown. The base port does not retry automatically.

**Rationale:** A process can perform side effects before observation fails.

### Decision: Output uses bounded content references

**Choice:** Capture bounded output prefixes through Bounded Exec, then store admitted output through the selected content-store port.

Canonical execution outcomes bind output content refs, observed byte counts, retained byte counts, truncation, stream role, and content receipts.

**Rationale:** Large or sensitive output must not enter callback records or unbounded memory.

### Decision: Generation and operation identity fence effects

**Choice:** Every request binds extension, service, generation, callback, effect, operation, executable, and idempotency identities.

Completions for stale generations, different operations, substituted executables, or incompatible profiles deny before callback delivery.

**Rationale:** Delayed process results must not mutate replacement service state.

### Decision: Simulation shares the command algebra

**Choice:** A deterministic adapter consumes the same admitted request and returns scripted canonical lifecycle observations.

The simulator models start refusal, output, exit, timeout, cancellation, host failure, and uncertain completion without spawning a process.

**Rationale:** Same-core conformance requires adapter substitution, not mock-only application logic.

## Required Flow

```text
extension effect request
  -> execution profile and authority admission
  -> artifact and workspace capability resolution
  -> durable intent or reservation when required
  -> bounded-exec live adapter or deterministic adapter
  -> bounded lifecycle and output observation
  -> content-store publication for retained output
  -> canonical execution receipt
  -> consuming extension outcome policy
```

## Test Design

Pure tests cover profile drift, unsupported environment modes, duplicate values, stale generations, overbound inputs, invalid lifecycle changes, and outcome linkage.

Live adapter tests cover accepted exits, rejected exits, standard input, output floods, timeout, cancellation, descendant-held pipes, teardown, and unavailable executables.

Negative authority tests prove that missing provenance, process authority, workspace authority, effect admission, or resources prevent spawn.

Simulation tests replay the same command and event sequence. They also inject refusal, timeout, cancellation, truncation, and uncertain completion.

Architecture tests reject process calls outside the adapter, port traits inside adapter modules, mutable sibling dependencies, raw string failures, and policy duplication.

## Risks and Trade-offs

- The initial profile does not provide a sandbox. Consumers must state and enforce separate sandbox requirements.
- Content publication adds an effect after capture. Publication failure cannot erase the process observation.
- Process cancellation cannot prove that external side effects did not occur.
- Platform teardown behavior differs. Profiles must expose the supported termination scope and non-claims.
- One process per callback can cost more than a persistent worker. The initial profile favors isolation and reviewability.

## Claim Boundary

A passing receipt proves only the recorded bounded process observation for one request, adapter, host, and profile.

It does not prove sandboxing, hermeticity, authorization correctness, child correctness, network isolation, platform equivalence, or release readiness.
