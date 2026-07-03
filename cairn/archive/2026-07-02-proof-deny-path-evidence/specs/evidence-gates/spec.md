## ADDED Requirements

### Requirement: Proof gates declare deny-path matrix
r[molten.evidence.proof_deny_matrix.catalog] Proof-bearing evidence gates SHOULD declare a deterministic deny-path matrix that lists required negative evidence classes, expected denial decisions, fixture or generated evidence refs, and mutation-boundary expectations.

#### Scenario: Gate summary lists required denials
- GIVEN a proof-bearing gate profile
- WHEN its evidence summary is rendered
- THEN the summary lists the negative classes required for release review.

### Requirement: Required negative fixtures fail closed
r[molten.evidence.proof_deny_matrix.fail_closed_fixtures] Proof-bearing gates SHOULD include negative fixtures for missing artifacts, stale refs, malformed schemas, tampered bytes, duplicate receipts, wrong signer, wrong purpose, denied mutation attempts, and diagnostic-only evidence.

#### Scenario: Missing artifact fixture denies
- GIVEN a gate fixture with a referenced artifact missing
- WHEN the gate validates the fixture
- THEN the gate emits deny evidence before accepting pass evidence.

### Requirement: Denials bind no-mutation evidence
r[molten.evidence.proof_deny_matrix.no_mutation_evidence] Gates that deny before side effects MUST bind no-mutation evidence or unchanged state refs when the gate's claim includes denial-before-mutation behavior.

#### Scenario: Denied mutation leaves state unchanged
- GIVEN a request that should be denied before mutation
- WHEN the gate evaluates the request
- THEN the denial receipt binds the before and after state refs or a no-mutation receipt proving no committed mutation occurred.

### Requirement: Schema and tamper cases deny
r[molten.evidence.proof_deny_matrix.schema_tamper_cases] Proof-bearing gates MUST reject malformed schemas, unsupported schema versions, tampered canonical bytes, and mismatched embedded refs before producing pass evidence.

#### Scenario: Tampered canonical bytes deny
- GIVEN a receipt whose embedded ref does not match its canonical bytes
- WHEN the gate validates the receipt
- THEN the gate emits deny evidence with a tamper diagnostic.

### Requirement: Signature tamper cases deny
r[molten.evidence.proof_deny_matrix.signature_tamper_cases] Gates that accept signed evidence MUST reject wrong signer, wrong purpose, wrong key, revoked key, malformed envelope, duplicated signed member, and mismatched subject refs.

#### Scenario: Wrong purpose cannot pass
- GIVEN a signed receipt envelope with a diagnostic-only purpose
- WHEN a pass-evidence gate requires a release purpose
- THEN the gate denies the evidence.

### Requirement: Diagnostic evidence stays non-pass
r[molten.evidence.proof_deny_matrix.diagnostic_only] Diagnostic-only receipts, logs, redacted diagnostic bundles, and failure repro bundles MUST NOT satisfy pass evidence gates unless a future policy explicitly marks the transform gate-preserving.

#### Scenario: Diagnostic bundle rejected as pass evidence
- GIVEN a redacted diagnostic bundle
- WHEN a pass evidence gate evaluates the bundle
- THEN it denies pass evidence and records the diagnostic-only reason.

### Requirement: Deny matrix Hegel properties
r[molten.evidence.proof_deny_matrix.hegel_properties] Deny-path validation SHOULD include Hegel RS property tests that generate malformed refs, schema drift, duplicated receipts, signer/purpose mismatches, tampered bytes, and denied mutation intents.

#### Scenario: Generated tamper case cannot pass
- GIVEN Hegel RS generates a valid gate input and a tampered variant
- WHEN both are validated
- THEN the tampered variant cannot produce pass evidence.

### Requirement: Deny matrix documentation
r[molten.evidence.proof_deny_matrix.docs] Release and contributor documentation SHOULD explain the deny-path matrix and how negative evidence supports proof review.

#### Scenario: Reviewer finds negative evidence
- GIVEN a reviewer inspects a proof gate summary
- WHEN they follow the documentation
- THEN they can locate canonical negative evidence refs for each required deny class.
