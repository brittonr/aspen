## ADDED Requirements

### Requirement: ChaosControl SMR profiles use immutable contracts
r[molten.testing.chaoscontrol_smr_profile] Molten MUST bind an immutable ChaosControl package revision, workload schema ref, hash profile ref, initial-state ref, observation mode, guest closure, Molten artifact refs, campaign profile, and evidence classes. Workspace-relative paths MUST NOT become evidence-bearing runtime inputs.

#### Scenario: Exact producer and consumer cohort is admitted
- GIVEN all producer, schema, guest, Molten, profile, and bound refs are complete and compatible
- WHEN external campaign preflight runs
- THEN it emits an exact cohort ref and bounded execution plan.

#### Scenario: Local sibling path is the only producer identity
- GIVEN configuration names a mutable sibling checkout without immutable source and schema refs
- WHEN evidence-bearing preflight runs
- THEN it rejects the cohort before materialization or KVM execution.

### Requirement: ChaosControl evidence imports fail closed
r[molten.testing.chaoscontrol_smr_evidence] Molten MUST validate external producer identity, schema, consumer artifacts, observer path, observation mode, dropped-event accounting, profile, bounds, fault outcomes, observation summary, safety, liveness preconditions, replay class, and non-claims. The importer MUST emit a canonical external-evidence receipt or a typed rejection.

#### Scenario: Complete external bundle imports
- GIVEN a bounded ChaosControl bundle matches the admitted Molten cohort and workload contract
- WHEN offline import runs
- THEN Molten emits a canonical receipt that preserves the external evidence role and source refs.

#### Scenario: Selected fault is presented as observed
- GIVEN a bundle records fault selection without matching applied and observed outcomes
- WHEN import validation runs
- THEN it rejects the effect claim or classifies the evidence as incomplete.

### Requirement: Simulation and VM claims remain separate
r[molten.testing.chaoscontrol_smr_claim_boundary] Molten MAY share operation corpora, invariant identifiers, and expected failure classes across whole-system simulation and ChaosControl. It MUST retain distinct evidence profiles. Neither profile MAY substitute for the other or grant authority, security, deployment, or release eligibility.

#### Scenario: Simulation and VM results match
- GIVEN both profiles produce matching bounded semantic results
- WHEN evidence is assembled
- THEN each receipt retains its own environment, adapter, fault, replay, and non-claim facts.

#### Scenario: VM receipt is promoted to production proof
- GIVEN a consumer labels one bounded KVM workload receipt as universal consensus correctness, production SLO evidence, or release eligibility
- WHEN claim admission runs
- THEN it rejects the promoted role.

### Requirement: ChaosControl integration preserves a functional core
r[molten.testing.chaoscontrol_smr_boundary] Chain projection inputs, operation-outcome mapping, external evidence admission, claim classification, and cross-profile comparison MUST be pure deterministic logic. Nix, files, processes, KVM, guest control, bundle reads, persistence, and output MUST remain in shells.

#### Scenario: Import decision runs in memory
- GIVEN parsed external evidence and expected cohort facts
- WHEN admission runs
- THEN it returns the same decision without filesystem, environment, process, wall-clock, network, or KVM access.

### Requirement: External consensus conformance covers success and failure
r[molten.testing.chaoscontrol_smr_validation] The change MUST include positive and negative tests for chain projection, observation completeness, operation identity, profiles, guest packaging, faults, recovery, liveness, evidence import, replay, and claim boundaries.

#### Scenario: Selected validation rail runs
- GIVEN valid no-fault and recovery fixtures plus deliberate divergence, duplicate, rollback, stall, malformed evidence, and overclaim fixtures
- WHEN validation runs
- THEN valid fixtures pass and each negative fixture fails with its expected stable class.
