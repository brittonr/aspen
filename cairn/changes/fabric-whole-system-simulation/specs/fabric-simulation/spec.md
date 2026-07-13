## ADDED Requirements

### Requirement: Simulated worlds are canonical and bounded
r[molten.fabric_simulation.world_manifest] Molten MUST define a canonical simulated-world manifest binding node descriptors, system-extension artifacts and generations, configuration and schemas, fabric-port profiles, initial durable state, membership views, placement and consistency profiles, authority and policy refs, resource envelopes, workload, scheduler and entropy inputs, fault plan, invariants, exploration bounds, runtime identity, evidence profile, and non-claims. Missing behavior-affecting inputs, incompatible profiles, unknown faults, or unbounded exploration MUST deny an evidence-bearing run.

#### Scenario: Complete world is admitted
- GIVEN a world manifest closes every extension and port dependency and declares finite scheduler, time, resource, workload, fault, and trace bounds
- WHEN simulation preflight runs
- THEN it produces a canonical world ref and admitted run plan.

#### Scenario: Ambient dependency denies deterministic admission
- GIVEN an extension or adapter would read undeclared filesystem, environment, network, wall-clock, entropy, process, or scheduler state
- WHEN deterministic preflight or execution detects the access
- THEN the run denies or fails with an ambient-input diagnostic
- AND cannot satisfy deterministic evidence gates.

### Requirement: Simulation runs the same extension core
r[molten.fabric_simulation.same_core] Molten MUST run the same system-extension artifact identity, manifest, callback dispatcher, pure protocol and state-transition core, application state-machine code, schemas, and canonical port command/event types in deterministic simulation and live execution. Simulation-specific replacement MUST be limited to admitted shell adapters and top-level scheduling. A mock-only reimplementation or fabricated callback receipt MUST NOT satisfy same-core conformance.

#### Scenario: Extension identity matches across profiles
- GIVEN one extension artifact is admitted for deterministic simulation and a live profile
- WHEN each composition loads it
- THEN the extension core, manifest, callback groups, schemas, and port-contract refs have the same canonical identities apart from declared shell bindings.

#### Scenario: Mock service fails same-core check
- GIVEN a test double directly implements expected service outputs without invoking the admitted extension callbacks and core
- WHEN same-core conformance runs
- THEN it fails even if output fixtures happen to match.

### Requirement: Every effect uses a substitutable fabric port
r[molten.fabric_simulation.port_substitution] Molten MUST provide deterministic adapters for the transport, durable-state, time, entropy, scheduler, membership, placement, consistency, process-lifecycle, supervision, resource, policy, and authority operations required by an admitted simulated world. Adapters MUST preserve their canonical live-port commands, events, transitions, bounds, failure classes, generation fencing, and non-claims. They MUST NOT mutate extension semantic state invisibly.

#### Scenario: Protocol core changes no code for simulation
- GIVEN an extension uses admitted transport, durable-state, and timer ports
- WHEN it runs in simulation
- THEN the same core submits and receives the same canonical command and event types through simulated bindings.

#### Scenario: Missing adapter denies world activation
- GIVEN an extension requires a port for which the world has no compatible deterministic adapter
- WHEN activation resolves bindings
- THEN activation denies instead of bypassing the port or performing live I/O.

### Requirement: One deterministic scheduler controls simulation choices
r[molten.fabric_simulation.scheduler] Molten MUST make runnable selection, message delivery, timer firing at eligible virtual time, storage completion, process lifecycle completion, fault activation, and other modeled nondeterministic choices explicit under one bounded deterministic scheduler. Each choice MUST have a canonical position, eligible-set ref, selected alternative, and replay behavior.

#### Scenario: Identical run inputs replay identically
- GIVEN identical world, runtime, scheduler, entropy, workload, and fault refs
- WHEN a deterministic run is repeated
- THEN canonical choices, events, semantic traces, outputs, invariant results, and final state refs match.

#### Scenario: Scheduler divergence stops replay
- GIVEN replay reaches a choice whose recorded alternative is not eligible
- WHEN the scheduler validates the position
- THEN replay stops at that first divergence
- AND does not silently choose a different event.

### Requirement: Faults occur only at named canonical boundaries
r[molten.fabric_simulation.fault_model] Molten MUST model faults as canonical actions at declared port or lifecycle boundaries. Supported profiles MAY include delay, drop, duplicate, reorder, partition, reset, bounded corruption, capacity exhaustion, pause, crash, restart, clock anomaly, timer delay, authority revocation, membership change, placement replacement, and consistency quorum loss. Faults MUST identify target, activation condition, duration or terminal rule, scope, resource cost, expected observability, and non-claims.

#### Scenario: Disk crash respects flush boundary
- GIVEN a fault crashes a node after buffered state but before its selected durable flush completion
- WHEN recovery runs
- THEN the durable-state adapter exposes only outcomes permitted by its profile.

#### Scenario: Fault cannot patch extension state
- GIVEN a fault plan attempts to mutate an extension's transaction table directly
- WHEN world validation runs
- THEN validation denies because the mutation does not cross a declared port or lifecycle boundary.

### Requirement: Invariants are pure and extension-owned where semantic
r[molten.fabric_simulation.invariants] Molten MUST evaluate pure bounded invariants over canonical state refs, histories, events, and redacted observations. Extensions MUST own semantic invariants such as transaction consistency, log-history rules, or scheduler completion rules. The fabric MUST provide universal invariants for no ambient effect, no stale-generation mutation, no port state-machine violation, no resource-bound bypass, no invalid canonical ref, and complete terminal cleanup.

#### Scenario: Transaction invariant finds a counterexample
- GIVEN a transactional extension history violates its declared isolation invariant under a generated schedule
- WHEN invariant evaluation reaches the violating prefix
- THEN the run fails at the earliest identified boundary with state, history, choice, and fault refs.

#### Scenario: Fabric does not invent service semantics
- GIVEN a replicated log extension defines offset and retention behavior
- WHEN the harness validates it
- THEN those rules come from extension-owned invariant artifacts rather than hard-coded node-core assumptions.

### Requirement: Replay and shrinking preserve causal validity
r[molten.fabric_simulation.replay_shrink] Molten MUST replay deterministic failures from canonical initial world, workload, scheduler, entropy, fault, runtime, and adapter refs and report the first semantic divergence. It SHOULD shrink workloads, fault actions, schedules, delays, resource envelopes, and eligible nodes while preserving world validity, the same failure class, and causal replay. Invalid shrink candidates MUST be rejected rather than repaired through hidden inputs.

#### Scenario: Shrunk failure becomes a standalone fixture
- GIVEN a deterministic run fails an invariant and shrinking finds a smaller causal case
- WHEN the result is exported
- THEN the minimal world, workload, fault plan, choice trace, runtime refs, expected failure, and first-divergence data form a replayable fixture.

#### Scenario: Invalid node reduction is rejected
- GIVEN a shrink step removes a node required by the world's declared membership or property precondition
- WHEN candidate validation runs
- THEN that candidate is rejected and not reported as a valid smaller counterexample.

### Requirement: Live and simulated adapters have differential conformance
r[molten.fabric_simulation.live_sim_differential] Molten MUST run shared adapter contracts and SHOULD compare canonical semantic traces between deterministic and live profiles for behavior their descriptors declare equivalent. Differences caused by declared adapter capabilities or nondeterministic live scheduling MUST be normalized only through reviewed rules and remain visible in profile metadata.

#### Scenario: Equivalent port trace matches
- GIVEN a bounded no-fault fixture whose live and simulated profiles declare equivalent transport and durability behavior
- WHEN differential conformance runs
- THEN canonical commands, outcomes, lifecycle transitions, and resulting extension state fall within the same allowed trace set.

#### Scenario: Hidden simulator shortcut fails
- GIVEN the simulator acknowledges durable completion earlier than the live contract permits
- WHEN differential and crash conformance run
- THEN the simulator profile fails with a durability-boundary diagnostic.

### Requirement: Reference services demonstrate fabric sufficiency
r[molten.fabric_simulation.reference_services] Molten SHOULD provide minimal system-extension vertical slices for a transactional ordered key-value service, a replicated append log, and a distributed scheduler. The key-value slice MUST own transaction, conflict, commit, and recovery semantics; the log slice MUST own offsets, retention, replication, and recovery semantics; the scheduler slice MUST own jobs, leases, retries, completion, and failover semantics. These slices MUST use system-extension callbacks and fabric ports and MUST NOT claim FoundationDB, Kafka, or external scheduler compatibility.

#### Scenario: Three distinct services run in simulation
- GIVEN admitted world manifests for all reference slices
- WHEN their workloads and fault plans execute
- THEN each service uses common fabric mechanisms while preserving its own state and history invariants.

#### Scenario: Reference behavior cannot bypass a missing port
- GIVEN a reference slice needs behavior not available through its admitted ports
- WHEN it activates or executes
- THEN it denies or records the missing fabric primitive
- AND it does not access ambient runtime internals.

### Requirement: Reference slices prove no-core-modification sufficiency
r[molten.fabric_simulation.fabric_sufficiency] Molten MUST maintain a conformance check showing that the reference key-value, log, and scheduler slices install, activate, run, recover, and drain without extension-specific node-core branches, direct adapter imports, ambient authority, or mock-only logic. Common missing mechanisms MUST be proposed as versioned fabric ports; workload semantics MUST remain in extensions.

#### Scenario: Node core remains workload-neutral
- GIVEN all three reference slices pass their bounded conformance suites
- WHEN code-boundary validation inspects their runtime paths
- THEN node core dispatches canonical system-extension and port operations without matching on database, log, or scheduler domain semantics.

#### Scenario: Domain branch in core fails conformance
- GIVEN node core contains a special transaction-offset or job-state branch used only to make one reference slice pass
- WHEN fabric-sufficiency validation runs
- THEN conformance fails and identifies the leaked domain semantic.

### Requirement: Simulation evidence follows an explicit claim ladder
r[molten.fabric_simulation.claim_ladder] Molten MUST classify evidence at least as pure model, deterministic whole-system simulation, multi-process live, host-chaos, or VM/hardware profile where those profiles are implemented. Promotion to a stronger claim MUST require that profile's own implementation identity, environment, adapter, lifecycle, fault, and operator evidence; deterministic simulation MUST NOT be relabeled as live or production evidence.

#### Scenario: Simulation evidence supports but does not replace live admission
- GIVEN a service has passing deterministic whole-system simulation evidence
- WHEN a production gate requires multi-process live evidence
- THEN simulation evidence may satisfy the simulation prerequisite
- AND the gate still requires live profile evidence.

#### Scenario: Missing profile evidence denies promotion
- GIVEN a report labels a run host-chaos but contains only deterministic adapter refs
- WHEN claim validation runs
- THEN it denies the stronger label with a profile mismatch.

### Requirement: Simulation evidence and workflows are bounded
r[molten.fabric_simulation.evidence] Molten MUST emit canonical world, run, scheduler-choice, fault, invariant, coverage, differential, divergence, shrink, final-state, and claim-profile evidence compatible with sealed reproducibility bundles and cluster run-directory conventions. Evidence MAY aggregate hot-path events but MUST preserve enough canonical history to replay and validate selected invariants.

#### Scenario: Passing run is offline-verifiable
- GIVEN a deterministic whole-system run completes within bounds
- WHEN its bundle is validated offline
- THEN refs bind the world, runtime, adapters, workload, choices, entropy, faults, invariants, coverage, final states, decision, and non-claims.

#### Scenario: Secret observations remain redacted
- GIVEN a simulation processes secret-marked fixture values
- WHEN status or a repro bundle is rendered
- THEN approved redacted markers or encrypted refs replace secret bytes without changing canonical source-evidence handling.

### Requirement: Operators can run and inspect whole-system simulations
r[molten.fabric_simulation.operator_workflow] Molten SHOULD provide bounded CLI or harness workflows to preflight, run, replay, shrink, inspect, and export a simulated world. Status MUST identify current virtual time, choice and event counts, node and service states, active faults, invariant state, resource use, first divergence, evidence refs, and claim profile without unbounded payload rendering.

#### Scenario: Preflight reports missing closure
- GIVEN a world omits a required adapter or schema
- WHEN an operator runs preflight
- THEN it reports the missing closure and performs no simulation effects.

### Requirement: Whole-system simulation validation covers success and failure
r[molten.fabric_simulation.final_validation] Molten MUST include positive and negative tests for world validation, same-core identity, port substitution, ambient-I/O denial, deterministic scheduler replay, each fault class, invariant success and failure, first divergence, shrinking, adapter differential conformance, reference services, stale generations, resource bounds, cleanup, sealed evidence, and claim-profile promotion.

#### Scenario: Deterministic reference run repeats
- GIVEN a valid reference-service world and bounded fault plan
- WHEN the run executes repeatedly with identical canonical inputs
- THEN semantic traces, invariant outcomes, final state refs, and report refs match.

#### Scenario: Fabric bypass fixture fails
- GIVEN an extension test path uses direct storage, socket, clock, or process access or a mock-only service implementation
- WHEN whole-system conformance runs
- THEN validation denies with the exact bypass or same-core invariant.
