## ADDED Requirements

### Requirement: Hegel proof-law catalog
r[molten.haskell_patterns.hegel_proof_properties.catalog] Molten SHOULD maintain an explicit Hegel RS proof-law catalog for core evidence invariants that are better checked by generated inputs than by single examples.

#### Scenario: Proof law is traceable
- GIVEN a Hegel RS property suite for a core evidence invariant
- WHEN traceability scans proof coverage
- THEN the suite is linked to the requirement id for that invariant.

### Requirement: Canonical ref stability property
r[molten.haskell_patterns.hegel_proof_properties.canonical_ref_stability] Core evidence code SHOULD have Hegel RS properties showing identical semantic inputs produce identical canonical refs and semantic binding changes produce changed refs or denial.

#### Scenario: Same generated input has same ref
- GIVEN Hegel RS generates a bounded canonical receipt input
- WHEN the receipt is rendered twice
- THEN both renderings produce the same canonical ref.

### Requirement: Traceability decision law
r[molten.haskell_patterns.hegel_proof_properties.traceability_decision_law] Traceability core SHOULD have Hegel RS properties proving the manifest decision is pass exactly when required positive coverage, required negative coverage, and stale-reference constraints are satisfied under policy.

#### Scenario: Missing generated negative coverage denies
- GIVEN Hegel RS generates a changed evidence-bearing requirement with positive coverage and no negative coverage
- WHEN the manifest is built
- THEN the manifest decision is deny with a missing-negative diagnostic.

### Requirement: Deny monotonicity property
r[molten.haskell_patterns.hegel_proof_properties.deny_monotonicity] Evidence validation SHOULD have Hegel RS properties proving that adding stale, malformed, mismatched, or tampered evidence cannot turn a denied proof into pass and cannot silently satisfy an unrelated requirement.

#### Scenario: Stale generated receipt cannot satisfy coverage
- GIVEN a generated coverage set and a stale receipt for a deleted requirement
- WHEN validation runs
- THEN validation reports stale evidence or excludes it by explicit policy without creating pass coverage.

### Requirement: Diagnostic evidence non-pass property
r[molten.haskell_patterns.hegel_proof_properties.diagnostic_nonpass_law] Gate validation SHOULD have Hegel RS properties proving diagnostic-only evidence cannot satisfy pass gates.

#### Scenario: Generated diagnostic receipt denied as pass
- GIVEN Hegel RS generates a diagnostic-only receipt shape
- WHEN a pass-evidence gate validates it
- THEN the gate denies pass evidence.

### Requirement: Replay compares canonical refs property
r[molten.haskell_patterns.hegel_proof_properties.replay_ref_law] Replay and drift checks SHOULD have Hegel RS properties proving semantic comparisons use canonical refs and declared variance, not rendered logs.

#### Scenario: Rendered log drift is not semantic drift
- GIVEN generated artifacts with the same canonical refs and different rendered diagnostic text allowed by variance
- WHEN drift comparison runs
- THEN the semantic decision follows canonical refs and declared variance.

### Requirement: Shrunk proof counterexamples are canonical
r[molten.haskell_patterns.hegel_proof_properties.shrink_fixture_receipts] Persisted Hegel RS counterexamples that cross proof, replay, or release boundaries MUST be represented as canonical fixture data with seed, shrink path, input, expected law, and relevant receipt refs.

#### Scenario: Persisted counterexample reruns
- GIVEN a shrunk Hegel RS counterexample saved for review
- WHEN it is imported into a repro or proof fixture
- THEN the fixture can rerun the exact generated input without ambient generator state.

### Requirement: Hegel property coverage appears in manifests
r[molten.haskell_patterns.hegel_proof_properties.coverage_manifest] Hegel RS proof-law suites SHOULD appear in traceability coverage manifests as positive or negative evidence for their linked requirements.

#### Scenario: Property suite covers invariant
- GIVEN a Hegel RS suite proving a canonical ref invariant
- WHEN traceability coverage is generated
- THEN the suite's verification receipt appears as evidence for the invariant requirement.

### Requirement: Hegel proof-law documentation
r[molten.haskell_patterns.hegel_proof_properties.docs] Contributor documentation SHOULD explain how to write bounded Hegel RS proof properties, name generator bounds, persist counterexamples, and link property evidence to requirements.

#### Scenario: Contributor adds generated proof law
- GIVEN a contributor adds a core evidence invariant
- WHEN they follow the documentation
- THEN they add Hegel RS positive and negative generated coverage with traceability refs.
