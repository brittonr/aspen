# Runtime Spine Delta: Provenance Admission Binding

### Requirement: Provenance records bind build record evidence
r[molten.provenance_admission_binding.spec.build_record_binding] Molten MUST let provenance records that claim reproducible verification bind explicit build record refs as part of their canonical provenance evidence.

#### Scenario: Reproducible provenance names build record refs
- GIVEN an artifact with a reproducible build record
- WHEN an operator materializes a reproducible provenance record
- THEN the record carries the expected build record refs
- AND those refs are part of the canonical provenance record hash.

### Requirement: Reproducible verification requires build receipts
r[molten.provenance_admission_binding.spec.verify_receipt_required] Molten MUST deny `reproducible-verified` provenance admission unless a supplied build verification receipt passes, names the evaluated artifact as expected and actual artifact, and references a build record ref bound by the provenance record.

#### Scenario: Self-asserted reproducible trust denies
- GIVEN a provenance record whose trust state is `reproducible-verified`
- WHEN evaluation receives no matching passing build verification receipt
- THEN Molten denies provenance admission
- AND diagnostics identify the missing or mismatched build verification evidence.

### Requirement: Build verification remains evidence-only
r[molten.provenance_admission_binding.spec.evidence_only] Molten MUST treat build verification receipts as provenance evidence only; they SHALL NOT grant authority, policy, resource, transport, execution, or source-gate trust.

#### Scenario: Build receipt does not grant authority
- GIVEN a passing build verification receipt
- WHEN node-control evaluates an install or run request
- THEN the receipt may satisfy reproducible provenance evidence
- AND node-control still requires independent authority, policy, resource, execution, transport, and source-gate evidence where applicable.

### Requirement: CLI accepts build verification evidence
r[molten.provenance_admission_binding.spec.cli_evaluate_build_evidence] The `molten test provenance evaluate` command MUST accept explicit build verification receipt inputs and include their refs in the emitted provenance evaluation receipt.

#### Scenario: CLI evaluates reproducible provenance
- GIVEN a provenance record and a matching build verification receipt file
- WHEN an operator runs provenance evaluation with both inputs
- THEN Molten emits a passing provenance receipt
- AND the receipt binds the build verification receipt ref considered during evaluation.

### Requirement: Node-control validates reproducible binding
r[molten.provenance_admission_binding.spec.node_control_binding] Node-control install and run gates MUST split provenance records from build verification receipts and deny reproducible provenance before side effects when build verification evidence is absent, denied, mismatched, or unbound.

#### Scenario: Node-control denies unbound build evidence
- GIVEN a node-control install request with `reproducible-verified` provenance
- WHEN its build verification receipt references a build record not bound by that provenance record
- THEN node-control denies the operation before mutating the registry
- AND emits provenance diagnostics as a subreceipt.

### Requirement: Provenance receipts bind build verification refs
r[molten.provenance_admission_binding.spec.receipt_refs] Provenance evaluation receipts SHOULD include the canonical refs of build verification receipts supplied to the evaluator so downstream diagnostics can audit the evidence considered.

#### Scenario: Receipt carries build verification refs
- GIVEN one or more build verification receipt inputs
- WHEN provenance evaluation emits a receipt
- THEN the receipt includes the canonical build verification receipt refs
- AND those refs remain non-authority evidence.

### Requirement: Matching build evidence admits reproducible provenance
r[molten.provenance_admission_binding.spec.matching_pass] Molten MUST include tests showing `reproducible-verified` provenance passes when the build verification receipt passes, matches the artifact, and references a bound build record ref.

#### Scenario: Matching receipt passes tests
- GIVEN a provenance record bound to a build record ref
- WHEN tests evaluate it with a matching passing build verification receipt
- THEN the provenance decision is pass
- AND node-control can proceed to the next independent gates.

### Requirement: Missing or mismatched build evidence denies
r[molten.provenance_admission_binding.spec.missing_or_mismatch_deny] Molten MUST include tests showing missing, denied, mismatched, malformed, or unbound build verification evidence denies `reproducible-verified` provenance.

#### Scenario: Missing receipt denies tests
- GIVEN self-asserted reproducible provenance with no build verification receipt
- WHEN tests evaluate it
- THEN the provenance decision is deny
- AND diagnostics mention required build verification evidence.
