## ADDED Requirements

### Requirement: Signed release promotion receipt
r[molten.evidence.release_promotion.signed_receipt] Molten's dogfood release evidence MUST sign the final `release-promotion-gate-receipt-v1` with a distinct `release-promotion` signed receipt purpose.

#### Scenario: Promotion receipt is signed after promotion passes
- GIVEN a dogfood release output with a passing release promotion gate receipt
- WHEN the dogfood release check finishes
- THEN it emits a signed receipt envelope for `release-promotion-gate.preserves`
- AND the signed envelope uses purpose `release-promotion`

### Requirement: Signed promotion receipt keyring verification
r[molten.evidence.release_promotion.signed_receipt_verify] Molten's dogfood release evidence MUST verify the signed release promotion receipt through the signed receipt keyring and fail the check when verification does not pass.

#### Scenario: Keyring verifies signed promotion receipt
- GIVEN a signed promotion receipt envelope and the dogfood signed receipt keyring
- WHEN signed receipt verification runs with the selected key id, signer, trust root, and purpose
- THEN verification passes
- AND the verification log is preserved with the dogfood release output

### Requirement: Signed promotion receipt remains evidence only
r[molten.evidence.release_promotion.signed_receipt_evidence_only] Signed release promotion receipts MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, release publication authority, or permission to bypass subsystem gates.

#### Scenario: Signed promotion is not release authority
- GIVEN a verified signed release promotion receipt
- WHEN a subsystem or release publisher requires its own authority or gate evidence
- THEN it MUST NOT treat the signed promotion receipt as sufficient authority
- AND it MUST still require the subsystem or publication gate evidence
