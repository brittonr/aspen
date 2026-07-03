## ADDED Requirements

### Requirement: Receipt-backed coverage source model
r[molten.testing.receipt_driven_traceability.source_model] Traceability SHOULD accept canonical proof receipt refs as coverage sources and treat hand-authored coverage tuples as compatibility-only input.

#### Scenario: Receipt ref is a coverage source
- GIVEN a verification or proof receipt that names a requirement id and coverage kind
- WHEN traceability scanning receives the receipt ref
- THEN the scanner derives a coverage entry from the canonical receipt fields.

### Requirement: Coverage is derived from receipts
r[molten.testing.receipt_driven_traceability.coverage_derivation] Receipt-driven traceability MUST derive requirement id, coverage kind, target, command identity, artifact refs, and diagnostics from validated canonical receipt fields rather than from rendered logs.

#### Scenario: Derived entry binds artifact refs
- GIVEN a receipt with produced artifact refs
- WHEN coverage derivation runs
- THEN the derived traceability entry names those refs and validates their content-ref shape.

### Requirement: Raw coverage claims are labeled
r[molten.testing.receipt_driven_traceability.raw_claim_policy] Traceability summaries MUST identify compatibility-only raw coverage entries and MAY allow release profiles to require receipt-backed coverage for changed evidence-bearing requirements.

#### Scenario: Raw tuple remains visible
- GIVEN a raw coverage tuple without a receipt ref
- WHEN the summary is rendered
- THEN the entry is labeled compatibility-only rather than indistinguishable from receipt-backed evidence.

### Requirement: Stale receipt coverage denies
r[molten.testing.receipt_driven_traceability.stale_receipt_denial] Receipt-driven traceability MUST deny stale receipt refs, duplicate coverage receipts for the same slot unless policy permits aggregation, wrong requirement ids, wrong coverage kinds, malformed refs, and receipts whose decision cannot satisfy the requested coverage kind.

#### Scenario: Wrong requirement receipt fails
- GIVEN a coverage slot for one requirement
- AND a receipt naming another requirement id
- WHEN traceability derives coverage
- THEN the slot remains uncovered and the receipt is reported as stale or mismatched.

### Requirement: Receipt-driven traceability has a gate surface
r[molten.testing.receipt_driven_traceability.nix_gate] Molten SHOULD expose receipt-driven traceability through the same release, Nix, or Cairn gate surface used by existing traceability scanning.

#### Scenario: Release gate requires receipt-backed coverage
- GIVEN a release profile that requires receipt-backed traceability
- WHEN a changed evidence-bearing requirement has only raw tuple coverage
- THEN the gate denies or marks the requirement as not release-covered.

### Requirement: Receipt-driven Hegel properties
r[molten.testing.receipt_driven_traceability.hegel_properties] Receipt-driven traceability SHOULD include Hegel RS property tests for deterministic derivation, positive/negative separation, duplicate handling, stale receipt denial, and deny-monotonicity when bad receipts are added.

#### Scenario: Adding stale receipt cannot create pass
- GIVEN Hegel RS generates a passing receipt-backed coverage set
- WHEN a stale receipt for a deleted requirement is added
- THEN the resulting traceability decision is deny or the stale receipt is explicitly excluded by policy with diagnostics.

### Requirement: Receipt-driven coverage docs
r[molten.testing.receipt_driven_traceability.docs] User-facing documentation SHOULD explain how to provide receipt refs to traceability and how compatibility-only raw tuples differ from receipt-backed coverage.

#### Scenario: Contributor migrates raw coverage
- GIVEN an existing raw coverage tuple
- WHEN a contributor follows the documentation
- THEN they can replace it with a receipt ref that derives the same requirement coverage.
