## ADDED Requirements

### Requirement: Molten is a workload-neutral distributed-systems fabric
r[molten.fabric_boundary.fabric_identity] Molten MUST define its core as a workload-neutral distributed-systems fabric that supplies canonical communication, lifecycle, authority, resource, execution, durability, transport, scheduling, supervision, and evidence mechanisms without defining one database, replicated-log, scheduler, actor, or workflow semantic as the product-wide default.

#### Scenario: Database semantics remain extension-owned
- GIVEN a system extension implements transactions, conflict detection, shards, and replicas
- WHEN reviewers inspect the node-core contract
- THEN the fabric exposes only the general ports and lifecycle mechanisms required by that extension
- AND database transaction semantics are not treated as global Molten behavior.

#### Scenario: Non-database service uses the same fabric
- GIVEN a replicated log or distributed scheduler selects admitted fabric ports
- WHEN the service is installed
- THEN it can use the same transport, durability, membership, scheduling, policy, resource, and simulation mechanisms without adopting database semantics.

### Requirement: Fabric mechanisms and extension semantics are separated
r[molten.fabric_boundary.mechanism_semantics_separation] Molten MUST keep mechanism contracts in the fabric and workload semantics in system extensions. Moving extension-specific semantics into the node core MUST require a separate reviewed change that identifies the general invariant, compatibility impact, authority impact, and reference-system evidence.

#### Scenario: Extension-specific offset policy stays outside core
- GIVEN a replicated-log extension defines consumer offsets and retention rules
- WHEN its implementation uses fabric durable-state and scheduling ports
- THEN offset and retention semantics remain owned by the extension
- AND the fabric does not infer those semantics for other services.

#### Scenario: Hidden semantic promotion is rejected
- GIVEN a node-core change makes one extension's transaction, ordering, retry, or retention rule mandatory for unrelated services
- WHEN architecture validation evaluates the change
- THEN validation denies unless a separate fabric requirement justifies the general mechanism
- AND diagnostics identify the leaked extension semantic.

### Requirement: Extension tiers have distinct authority
r[molten.fabric_boundary.extension_tiers] Molten MUST distinguish sandboxed plugins, system extensions, and applications or workloads. System-extension authority for long-lived services, protocol ownership, durable state, timers, membership, placement, or consistency MUST NOT be inferred from ordinary plugin installation or artifact possession.

#### Scenario: System extension receives reviewed service authority
- GIVEN an artifact has a system-extension manifest, passing policy and provenance evidence, explicit port bindings, resource grants, and lifecycle admission
- WHEN the node activates the extension
- THEN only the declared system-extension capabilities become available
- AND ordinary plugin capabilities remain narrower.

#### Scenario: Sandboxed plugin cannot claim system authority
- GIVEN a sandboxed plugin declares a storage or network-shaped operation string
- WHEN it lacks a system-extension profile and matching capability grants
- THEN activation or hostcall admission denies protocol ownership, durable-state ownership, timers, membership, placement, and consistency access.

### Requirement: Fabric capability ports are canonical and fail closed
r[molten.fabric_boundary.port_registry] Molten MUST represent fabric ports with canonical descriptors and registry entries that bind port id, version, operation classes, input and output schema refs, authority requirements, resource requirements, determinism class, replay class, implementation profile, conformance refs, and non-claims. Unknown, duplicate, incompatible, disabled, or silently substituted ports MUST deny before extension activation.

#### Scenario: Compatible port binding passes
- GIVEN a system extension requires a supported transport port version with matching schemas, authority, resources, and conformance evidence
- WHEN activation resolves the port through the registry
- THEN the binding passes and emits a canonical binding receipt naming the selected implementation profile.

#### Scenario: Silent fallback is rejected
- GIVEN an extension requests an unsupported durable-state port version
- WHEN the registry contains another version or adapter with different semantics
- THEN activation denies instead of silently selecting the other port
- AND diagnostics identify the unsupported version and available reviewed profiles.

### Requirement: Fabric evidence is emitted at bounded semantic boundaries
r[molten.fabric_boundary.evidence_granularity] Molten MUST emit canonical evidence at declared trust, lifecycle, commit, checkpoint, failure, and operator-observation boundaries while allowing bounded aggregate evidence for internal hot-path operations. A fabric profile MUST NOT require one heavyweight receipt for every internal page read, packet, scheduler poll, or cache lookup unless a reviewed security or debugging profile explicitly selects that behavior.

#### Scenario: Commit boundary emits aggregate evidence
- GIVEN a system extension processes many internal storage and transport operations for one admitted commit
- WHEN the semantic commit completes
- THEN the extension emits evidence binding the admitted inputs, resulting state or output refs, relevant durable and quorum boundaries, and aggregate diagnostics
- AND internal operations need not each become independent authority receipts.

#### Scenario: Debug profile does not become production default
- GIVEN a diagnostic profile records every internal operation
- WHEN a production profile is selected
- THEN the diagnostic evidence granularity is not enabled implicitly
- AND production resource admission remains bounded by its reviewed profile.

### Requirement: Diverse reference systems demonstrate fabric sufficiency
r[molten.fabric_boundary.reference_system_exit_criteria] Molten SHOULD maintain capability and conformance matrices showing that a transactional key-value service, a replicated log, and a distributed scheduler can be implemented as system extensions without modifying node-core semantics or bypassing authority, resource, durability, transport, scheduling, or simulation ports.

#### Scenario: Three reference systems use common mechanisms
- GIVEN reviewed reference designs for the three service classes
- WHEN their required capabilities are compared
- THEN common needs map to fabric ports
- AND transaction isolation, log-offset behavior, and scheduling policy remain extension-specific.

#### Scenario: Missing general primitive is visible
- GIVEN a reference system requires behavior unavailable through reviewed fabric ports
- WHEN its conformance matrix is evaluated
- THEN the matrix reports the missing primitive
- AND the system cannot claim fabric conformance through direct ambient access.

### Requirement: Fabric non-claims are explicit
r[molten.fabric_boundary.non_claims] Molten MUST state that fabric descriptors, bindings, receipts, simulations, and reference-system matrices do not by themselves prove database correctness, global ordering, global consensus, transport delivery, durable persistence, Byzantine tolerance, protocol compatibility, production readiness, or extension semantic correctness.

#### Scenario: Port binding is not behavioral proof
- GIVEN an extension has passing bindings for transport, durability, membership, and consistency ports
- WHEN a release gate evaluates extension correctness
- THEN the bindings count only as mechanism and admission evidence
- AND separate implementation, conformance, simulation, and operational evidence remains required.

### Requirement: Fabric boundary validation covers positive and negative paths
r[molten.fabric_boundary.final_validation] Molten MUST include positive and negative validation for fabric identity, extension tiers, port registration, profile compatibility, evidence granularity, reference-system matrices, and non-claim enforcement.

#### Scenario: Valid fabric fixture passes
- GIVEN a system-extension fixture uses compatible registered ports and preserves extension-owned semantics
- WHEN focused validation runs
- THEN validation passes with canonical descriptor and binding refs.

#### Scenario: Semantic leakage fixture denies
- GIVEN a fixture grants system authority to an ordinary plugin or treats one reference service's semantics as global fabric behavior
- WHEN focused validation runs
- THEN validation denies with a tier or mechanism-semantics diagnostic.
