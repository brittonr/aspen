# Molten World Commit Specification Delta

## Purpose

Define explicit logical and opaque execution-snapshot profiles for complete restore and bounded copy-on-write branching.

## ADDED Requirements

### Requirement: Execution snapshots use closed profile classes

r[molten.world_snapshot.profiles] Molten MUST distinguish logical and opaque execution-snapshot profiles. Each profile MUST declare required roots, ownership, completeness, compatibility, restore, replay, retention, and merge behavior. Unknown profiles MUST deny.

#### Scenario: Logical profile is complete

- GIVEN every declared Molten logical root is present under one profile
- WHEN profile validation runs
- THEN Molten MUST classify the profile by its logical restore contract

#### Scenario: Unknown profile is supplied

- GIVEN a descriptor names an unsupported profile class
- WHEN validation runs
- THEN Molten MUST reject it instead of treating it as logical or opaque

### Requirement: Logical profiles bind resumable Molten state

r[molten.world_snapshot.logical] A logical profile MUST bind exact durable-state, task, history, scheduler, virtual-time, entropy, effect-state, runtime, schema, and policy roots required by its declared resume boundary.

#### Scenario: Required task root is absent

- GIVEN a logical profile declares resumable tasks but omits the task root
- WHEN completeness validation runs
- THEN Molten MUST classify the snapshot as incomplete

### Requirement: Opaque profiles require exact machine completeness

r[molten.world_snapshot.opaque] An opaque profile MUST bind an exact machine-snapshot descriptor and complete CPU, memory, device, disk, backend, topology, and runtime cohort facts. Missing or incompatible facts MUST block restore.

#### Scenario: Exact ChaosControl cohort matches

- GIVEN the snapshot inventory and restore host observations match the admitted exact cohort
- WHEN opaque compatibility runs
- THEN Molten MAY produce an opaque restore plan

#### Scenario: CPU state inventory differs

- GIVEN the restore host lacks one required CPU state group or feature inventory entry
- WHEN compatibility runs
- THEN Molten MUST deny restore without filling defaults

### Requirement: Compatibility binds behavior-relevant cohorts

r[molten.world_snapshot.cohort] Molten MUST compare exact architecture, runtime, ABI, schema, topology, device, storage, scheduler, time, entropy, and effect profile identities required by the selected snapshot class.

#### Scenario: Snapshot bytes match but runtime cohort differs

- GIVEN an object identity is valid but one required runtime behavior identity differs
- WHEN compatibility admission runs
- THEN Molten MUST reject the snapshot for that restore target

### Requirement: Copy-on-write children remain parent-bound and isolated

r[molten.world_snapshot.cow] Molten MUST use reviewed VM Cohort mechanics for opaque checkpoint clones. Each child MUST bind one parent and isolated memory, device, disk, and endpoint overlay identities.

#### Scenario: Child overlays are isolated

- GIVEN two children derive from one admitted checkpoint with distinct overlay identities
- WHEN clone validation runs
- THEN writes from either child MUST NOT be attributed to the other child or parent

#### Scenario: Overlay identity collides

- GIVEN a proposed child reuses an active sibling overlay identity
- WHEN clone admission runs
- THEN Molten MUST deny the clone before host effects

### Requirement: Restore recreates handles and rechecks authority

r[molten.world_snapshot.restore] Snapshot bytes MUST exclude live host handles and bearer authority. Restore MUST obtain new handles and recheck current policy, capability, revocation, resource, adapter, and runtime facts before activation.

#### Scenario: Snapshot is complete but authority expired

- GIVEN all deterministic state restores but current authority admission denies
- WHEN activation runs
- THEN Molten MUST keep the restored world inactive

#### Scenario: Descriptor contains a live handle

- GIVEN a candidate descriptor includes a file descriptor, socket handle, credential, or private key
- WHEN safe schema validation runs
- THEN Molten MUST reject or omit that field according to the closed schema

### Requirement: Snapshot verification covers completeness and overclaims

r[molten.world_snapshot.verification] Molten MUST test logical and opaque success paths plus incomplete state, cohort mismatch, unsafe handle, stale authority, isolation, merge denial, portability, and correctness overclaims.

#### Scenario: Focused snapshot rail runs

- GIVEN positive and negative fixtures use reviewed Molten, ChaosControl, and available VM Cohort cohorts
- WHEN the focused verification rail runs
- THEN it MUST report supported restore profiles and exact compatibility non-claims
