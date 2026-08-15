## ADDED Requirements

### Requirement: Provenance trust-state admission is profile-specific
r[molten.provenance_state_proof.profile_thresholds] Molten MUST prove that each provenance-sensitive operation profile admits only records at or above the required trust state and denies missing, denied, stale, or weaker trust states before side effects.

#### Scenario: Reviewed record denied for sensitive operation
- GIVEN an artifact with only a reviewed provenance record
- WHEN a sensitive operation requires reproducible-verified or policy-trusted provenance
- THEN provenance admission decision is `deny`
- AND no install, execution, remote sync, or policy mutation side effect is admitted.

### Requirement: Reproducible provenance binds build verification
r[molten.provenance_state_proof.build_verification_binding] Molten MUST prove that a reproducible-verified provenance record is admitted only when a passing build verification receipt binds the same artifact ref and build record ref carried by the provenance record.

#### Scenario: Wrong artifact build verification denies
- GIVEN a reproducible provenance record for artifact `A`
- WHEN the supplied build verification receipt verifies artifact `B`
- THEN provenance admission decision is `deny`
- AND diagnostics identify the artifact or build-record mismatch.

### Requirement: Provenance receipts remain non-authorizing
r[molten.provenance_state_proof.evidence_only_boundary] Molten MUST prove that provenance and build verification receipts do not replace authority, policy, resource, source-gate, transport, retention, or execution gates.

#### Scenario: Provenance alone cannot install
- GIVEN a passing provenance receipt and missing authority or source-gate evidence
- WHEN node control evaluates install admission
- THEN install admission is `deny`
- AND diagnostics identify the missing non-provenance gates.
