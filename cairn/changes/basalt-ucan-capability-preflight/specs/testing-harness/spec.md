# Testing Harness Delta: Basalt/UCAN capability preflight

### Requirement: Capability gates include Basalt authority preflight receipts
r[molten.testing.basalt_ucan_capability_preflight.authority_receipt] Evidence-bearing capability gates MUST include a Basalt authority contract envelope and preflight receipt bound to the canonical capability context ref.

#### Scenario: Valid authority preflight evidence
- GIVEN a suite with an explicit capability fixture
- WHEN the harness runs the suite
- THEN the report capability gate includes an authority contract envelope and `<basalt-authority-preflight ...>` receipt
- AND report validation recomputes the same authority preflight evidence from the embedded suite

#### Scenario: Missing authority preflight fails validation
- GIVEN a report whose capability gate lacks the Basalt authority preflight receipt
- WHEN report validation runs
- THEN validation fails closed before accepting the report as pass evidence

#### Scenario: Tampered authority preflight fails validation
- GIVEN a report whose authority preflight decision, reason, envelope ref, or capability ref has been modified
- WHEN report validation runs
- THEN validation rejects the report rather than trusting marker-only authority evidence

### Requirement: UCAN proofsets are explicit and bound
r[molten.testing.basalt_ucan_capability_preflight.proofset_binding] Capability gates MUST include an explicit UCAN proofset value and MUST bind its canonical ref into the Basalt authority preflight receipt. Empty local proofsets MAY satisfy the local harness. Non-empty proofsets MUST fail closed until full proof validation is implemented.

#### Scenario: Explicit empty local proofset passes
- GIVEN a deterministic local harness report with explicit capabilities
- WHEN validation checks capability evidence
- THEN an empty `<ucan-proofset-v1 ... []>` may satisfy local preflight

#### Scenario: Non-empty proofset fails closed
- GIVEN a report whose capability gate carries an unchecked UCAN proof ref
- WHEN report validation runs
- THEN validation rejects the report with a UCAN proof validation diagnostic

### Requirement: Grant refs bind admission authority
r[molten.testing.basalt_ucan_capability_preflight.grant_ref_binding] Capability gates MUST bind the ordered canonical grant refs from the embedded suite. Admission authority evidence that uses a grant ref MUST refer to a grant present in the Basalt authority preflight receipt.

#### Scenario: Grant refs match explicit fixture
- GIVEN an explicit capability fixture with grants
- WHEN the harness emits a report
- THEN the capability preflight receipt includes canonical refs for those grants
- AND each authorized admission decision references one of those refs

#### Scenario: Tampered grant ref fails validation
- GIVEN a report whose authority preflight grant refs have been modified
- WHEN report validation runs
- THEN validation fails closed before gate acceptance

### Requirement: Gate receipts expose capability preflight refs
r[molten.testing.basalt_ucan_capability_preflight.gate_receipts] Successful pass-evidence gate receipts MUST include checks and artifact refs for Basalt authority receipt, UCAN proofset binding, and grant-ref binding.

#### Scenario: Successful gate receipt includes authority checks
- GIVEN a deterministic report that validates and replays successfully
- WHEN `molten test gate check` emits a receipt
- THEN the receipt includes `basalt-authority-receipt`, `capability-proofset-binding`, and `grant-ref-binding` checks
