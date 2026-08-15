# Runtime Spine Delta: Supply-Chain Provenance UX

### Requirement: CLI emits canonical provenance records
r[molten.provenance_ux.spec.record_fixture_cli] The provenance UX CLI MUST emit canonical Preserves provenance records using the same record construction, validation, and hashing rules as the runtime provenance module.

#### Scenario: Reviewed fixture is materialized
- GIVEN an artifact ref
- WHEN an operator runs `molten test provenance fixture`
- THEN Molten emits a `provenance-record-v1` artifact with reviewed trust state
- AND the printed provenance ref is the canonical ref of that artifact.

### Requirement: CLI evaluation emits provenance receipts
r[molten.provenance_ux.spec.evaluate_receipts] The provenance UX CLI MUST evaluate explicit provenance record files with the same trust-state admission rules used by node-control provenance gates and MUST emit canonical `provenance-receipt-v1` receipts.

#### Scenario: Sandbox-only provenance is denied for node control
- GIVEN a provenance record with sandbox-only trust state for an artifact
- WHEN an operator evaluates it with profile `node-control`
- THEN Molten emits a denying provenance receipt
- AND the receipt diagnostics explain that sandbox-only trust is not admitted for node-control.

### Requirement: Provenance UX receipts are evidence only
r[molten.provenance_ux.spec.evidence_only] Provenance UX receipts MUST NOT grant authority, policy, resource, transport, execution, or source-gate trust.

#### Scenario: Provenance receipt is not a grant
- GIVEN a passing provenance evaluation receipt
- WHEN another subsystem needs authority, policy, resource, transport, execution, or source-gate admission
- THEN it must still resolve explicit evidence for that boundary
- AND it must not treat the provenance receipt as any non-provenance grant.
