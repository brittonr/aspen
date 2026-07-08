## ADDED Requirements

### Requirement: Consensus engine capabilities are trait-separated
r[molten.consensus_engine_traits.capability_traits] Molten SHOULD expose consensus engine behavior through separate capability traits for descriptor/readback, proposal transitions, reads, snapshots, and recovery rather than requiring one monolithic trait for every operation.

#### Scenario: Engine declares only supported capabilities
- GIVEN a consensus engine descriptor for an engine that supports reads but not snapshots
- WHEN production admission evaluates the engine
- THEN supported capabilities are explicit
- AND unsupported snapshot behavior cannot be inferred from a no-op trait method.

### Requirement: Proposal transitions have a pure core
r[molten.consensus_engine_traits.pure_transition_core] Consensus proposal admission and state-machine transition logic MUST be testable as a deterministic pure core that performs no filesystem, network, process, clock, scripting, runtime scheduling, or durable-store mutation.

#### Scenario: Invalid command does not mutate runtime
- GIVEN an invalid command envelope and a cloned prior control-registry state
- WHEN the pure transition core evaluates the input
- THEN it returns a denial result
- AND the prior state remains unchanged.

### Requirement: Runtime mutation is a thin consensus shell
r[molten.consensus_engine_traits.imperative_shell] Consensus runtime shells MUST apply mutations, persistence, and receipt storage only after the pure transition core returns an admitted transition result.

#### Scenario: Passing transition updates shell after core
- GIVEN a pure proposal transition result with decision `pass`
- WHEN the runtime shell applies the result
- THEN it updates runtime state and receipt indexes according to the returned canonical values
- AND it does not recompute divergent transition semantics.

### Requirement: Unsupported engine capabilities deny explicitly
r[molten.consensus_engine_traits.unsupported_denials] Molten MUST deny unsupported consensus engine capabilities with canonical diagnostics rather than silently treating missing trait support as success.

#### Scenario: Unsupported recovery denies
- GIVEN an engine descriptor that does not admit recovery
- WHEN recovery is requested for that engine
- THEN Molten emits a deny receipt or error diagnostic before any state replacement.

### Requirement: Trait refactors preserve canonical consensus evidence
r[molten.consensus_engine_traits.hash_stability] Refactoring consensus traits MUST preserve canonical command, registry, commit, read, snapshot, and recovery refs for equivalent semantic inputs unless the change records an explicit schema migration.

#### Scenario: Duplicate proposal ref remains stable
- GIVEN a duplicate client-sequence fixture before trait decomposition
- WHEN the same fixture runs through the decomposed traits
- THEN the duplicate denial or replay receipt ref is unchanged or the migration note records the intentional difference.

### Requirement: Consensus conformance binds declared capabilities
r[molten.consensus_engine_traits.conformance] Consensus conformance receipts or tests SHOULD assert that declared engine capabilities match the implemented trait support for descriptor/readback, proposal transitions, reads, snapshots, and recovery.

#### Scenario: Declared capability is implemented
- GIVEN a consensus engine descriptor that declares a proposal capability
- WHEN conformance checks exercise the engine through the decomposed trait surface
- THEN the proposal transition trait is implemented and returns canonical transition evidence
- AND unsupported capabilities still deny explicitly.
