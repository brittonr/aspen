## ADDED Requirements

### Requirement: Consensus engines are registered by explicit profile identity
r[molten.consensus.engine_registry] Molten MUST maintain an explicit consensus engine registry keyed by algorithm profile id and admitted profile version. Each registry entry MUST declare implementation id, supported read consistency modes, quorum/currentness evidence classes, membership/config capabilities, production-admission status, required evidence refs, conformance receipt refs, and operator caveats.

#### Scenario: Registered Raft profile resolves
- GIVEN a control-plane group manifest names the admitted Raft algorithm profile and profile version
- WHEN runtime construction resolves the manifest through the consensus engine registry
- THEN the registry returns the Raft engine descriptor
- AND the descriptor binds supported read modes, quorum evidence class, membership capability, conformance refs, and production-admission status.

#### Scenario: Unknown engine profile denies
- GIVEN a control-plane group manifest names an unknown, disabled, or misspelled algorithm profile
- WHEN runtime construction resolves the manifest through the consensus engine registry
- THEN Molten denies runtime construction before opening control-plane state
- AND diagnostics name the unsupported profile without falling back to another engine.

### Requirement: Consensus engines implement a common control-plane interface
r[molten.consensus.engine_interface] Molten MUST define a common control-plane consensus engine boundary for proposals, linearizable reads, local-stale reads, snapshots, recovery, membership/config transitions, placement validation, and readback summaries. Engine implementations MUST return canonical commit, denial, read, snapshot, recovery, and diagnostic receipts that preserve common evidence fields while allowing engine-specific evidence refs.

#### Scenario: Proposal uses normalized commit receipt
- GIVEN a valid control-plane command and an admitted engine descriptor
- WHEN Molten proposes the command through the engine interface
- THEN the engine returns a normalized commit or denial receipt
- AND the receipt binds operation id, state-machine id, engine profile, quorum/currentness evidence, command ref, and resulting state ref.

#### Scenario: Engine-specific evidence stays attached
- GIVEN an admitted engine requires algorithm-specific evidence such as term/index, quorum certificate, ballot, view, or proof ref
- WHEN the engine emits a normalized receipt
- THEN common receipt fields remain stable for coordination and policy gates
- AND engine-specific evidence is attached as opaque evidence refs without being interpreted by engine-agnostic callers.

### Requirement: Runtime engine selection is manifest-driven and fail-closed
r[molten.consensus.runtime_engine_selection] Molten MUST construct control-plane runtimes by resolving the group manifest's algorithm profile through the consensus engine registry rather than hard-coding a single Raft-only runtime path. Runtime construction MUST deny profiles that are not registered, not policy-admitted for the requested environment, or missing required evidence refs.

#### Scenario: Manifest selects admitted production engine
- GIVEN a group manifest names a registered production-admitted engine profile with complete evidence refs
- WHEN Molten constructs the control-plane runtime
- THEN runtime construction selects the matching engine implementation
- AND readback reports the selected engine, profile version, production status, and evidence refs.

#### Scenario: Experimental engine cannot start as production
- GIVEN a group manifest requests an experimental leaderless engine profile for production use
- WHEN runtime construction checks policy and evidence admission
- THEN construction denies unless the profile has accepted production-admission policy and all required proof, simulation, placement, membership, and operator evidence
- AND diagnostics identify each missing admission class.

### Requirement: Engine admission policy is auditable
r[molten.consensus.engine_admission_policy] Molten MUST require auditable policy evidence before a consensus engine registry entry can be marked production-admitted. Admission evidence MUST include implementation identity, profile version, supported consistency modes, failure model, non-claim boundaries, conformance receipts, placement requirements, membership requirements, and proof/model or simulation evidence appropriate to the algorithm.

#### Scenario: Complete admission evidence admits engine
- GIVEN an engine registry entry has accepted implementation identity, policy refs, conformance receipts, placement requirements, membership requirements, and proof or simulation evidence
- WHEN engine admission policy evaluates the entry
- THEN the entry may be marked production-admitted for the declared environment
- AND readback includes the evidence refs used for that admission.

#### Scenario: Missing conformance evidence denies admission
- GIVEN an engine registry entry omits required conformance or proof/model evidence
- WHEN engine admission policy evaluates the entry
- THEN production admission denies
- AND diagnostics name the missing evidence class and profile id.

### Requirement: Consensus application state is engine-portable
r[molten.consensus.engine_portable_state] Molten MUST keep control-plane application state machines independent from consensus engine internals. Application state transitions MUST consume canonical command/log envelopes and normalized commit/read evidence, and MUST NOT infer application semantics from engine-specific leader identity, term, view, ballot, transport path, replica locality, or local cache state.

#### Scenario: Same canonical commands produce same state across engines
- GIVEN two admitted engine simulations apply the same canonical control-plane command sequence to the same application manifest
- WHEN each engine emits normalized commit receipts for the sequence
- THEN Molten derives the same application state ref and application receipt refs
- AND any engine-specific evidence remains outside the application-state hash.

#### Scenario: Engine internals cannot authorize application mutation
- GIVEN an engine-specific local observation lacks a normalized commit or linearizable read receipt
- WHEN a control-plane application mutation guard evaluates the observation
- THEN the mutation guard denies the observation as insufficient evidence
- AND diagnostics require normalized consensus evidence.

### Requirement: Consensus engine switchover requires canonical receipts
r[molten.consensus.engine_switchover_receipts] Molten MUST require a canonical switchover plan and receipt before changing an existing control-plane group from one consensus engine/profile to another. The receipt MUST bind source profile, target profile, source state ref, target bootstrap state ref, membership/config refs, placement refs, fencing epoch, replay/conformance evidence, currentness evidence, operator approval refs, rollback posture, decision, and diagnostics.

#### Scenario: Safe switchover emits committed transition receipt
- GIVEN an existing control-plane group has current linearizable source-state evidence and a target engine profile with admitted policy, placement, membership, and conformance evidence
- WHEN the operator executes an approved switchover plan
- THEN Molten emits a committed switchover receipt
- AND the receipt binds source state, target bootstrap state, fencing epoch, replay evidence, and rollback posture.

#### Scenario: Unsafe switchover denies before target activation
- GIVEN a switchover plan has stale source-state evidence, missing target admission, incompatible membership, missing placement evidence, or failed replay/conformance evidence
- WHEN Molten evaluates the plan
- THEN Molten denies target activation before accepting new writes through the target engine
- AND diagnostics name each failed switchover precondition.

### Requirement: Engine switchover fences stale writers and readers
r[molten.consensus.engine_switchover_fencing] Molten MUST fence stale source-engine writers and stale target-engine readers during and after a consensus engine switchover. Mutation and linearizable-read receipts MUST bind the active engine epoch so downstream gates can reject receipts from inactive or superseded epochs.

#### Scenario: Old engine write is fenced after switchover
- GIVEN a switchover receipt has activated a target engine epoch
- WHEN a delayed source-engine proposal receipt arrives for the prior epoch
- THEN Molten rejects the receipt for current mutation authority
- AND diagnostics identify the stale engine epoch.

#### Scenario: Target read waits for activated epoch
- GIVEN a target engine has bootstrap state but the switchover receipt is not committed
- WHEN a client requests a linearizable control-plane read from the target engine
- THEN Molten denies the read as production currentness evidence
- AND diagnostics require the committed switchover epoch.
