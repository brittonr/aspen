## ADDED Requirements

### Requirement: Proof obligation manifests
r[molten.testing.proof_obligations.manifest] Molten SHOULD represent broad proof claims as deterministic proof-obligation manifests that list child obligations, subject refs, prerequisite refs, receipt refs, decisions, diagnostics, and evidence-only caveats.

#### Scenario: Aggregate proof lists child obligations
- GIVEN a workflow proof that depends on multiple semantic checks
- WHEN Molten renders the aggregate proof manifest
- THEN the manifest names each child obligation and the canonical receipt refs that satisfy it.

### Requirement: Standard proof obligation classes
r[molten.testing.proof_obligations.classes] Proof obligation manifests SHOULD distinguish input-validation, canonicalization, admission, mutation-boundary, replay-determinism, and fail-closed-negative obligations when those classes are part of a workflow claim.

#### Scenario: Mutation boundary is separate from admission
- GIVEN a workflow that denies an operation before mutation
- WHEN its proof manifest is rendered
- THEN admission evidence and no-mutation evidence appear as separate obligations.

### Requirement: Aggregate obligation gate
r[molten.testing.proof_obligations.aggregate_gate] Aggregate proof validation MUST fail closed when a required child obligation is missing, duplicated, bound to the wrong subject, bound to the wrong prerequisite, or has the wrong expected decision.

#### Scenario: Missing replay obligation denies aggregate proof
- GIVEN an aggregate proof requiring replay-determinism evidence
- WHEN the replay obligation receipt is absent
- THEN aggregate validation emits deny evidence for the missing child obligation.

### Requirement: Traceability can reference aggregate proofs
r[molten.testing.proof_obligations.traceability] Traceability MAY accept aggregate proof manifest refs as coverage evidence when the manifest exposes matching requirement ids and positive or negative coverage kinds.

#### Scenario: Requirement coverage comes from child obligation
- GIVEN an aggregate proof manifest with child obligations linked to requirement ids
- WHEN traceability consumes the manifest
- THEN the requirement is covered only by matching child obligations, not by the aggregate label alone.

### Requirement: Obligation summaries are operator-readable
r[molten.testing.proof_obligations.operator_summary] Proof obligation readbacks SHOULD group obligations by class, decision, subject, and missing or stale diagnostics.

#### Scenario: Summary names missing child
- GIVEN an aggregate proof manifest missing a mutation-boundary child
- WHEN the operator summary is rendered
- THEN it names the missing obligation class and subject ref.

### Requirement: Obligation Hegel properties
r[molten.testing.proof_obligations.hegel_properties] Proof obligation validation SHOULD include Hegel RS property tests for deterministic ordering, stable refs, missing-child denial, duplicate-child denial, mismatched-subject denial, and positive/negative substitution denial.

#### Scenario: Generated duplicate child denies
- GIVEN Hegel RS generates an aggregate manifest with duplicate child obligation ids
- WHEN aggregate validation runs
- THEN validation denies the aggregate proof.

### Requirement: Obligation fixtures
r[molten.testing.proof_obligations.fixtures] Proof obligation tests SHOULD include complete positive fixtures and negative fixtures for missing child, duplicate child, wrong subject, wrong prerequisite, stale receipt, and wrong expected decision.

#### Scenario: Wrong subject fixture fails
- GIVEN a child obligation receipt for a different subject ref
- WHEN aggregate validation runs
- THEN the aggregate proof is denied before satisfying coverage.

### Requirement: Obligation decomposition docs
r[molten.testing.proof_obligations.docs] Proof workflow documentation SHOULD explain how to decompose broad claims into child obligations and how aggregate proof manifests remain evidence-only.

#### Scenario: Contributor decomposes workflow claim
- GIVEN a contributor adds a new workflow proof
- WHEN they follow the documentation
- THEN they identify child obligations and attach explicit positive and negative receipts for review.
