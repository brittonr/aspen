# Evidence Gates Delta: Basalt/UCAN authority hardening

### Requirement: Basalt/UCAN authority receipts bind all admission inputs
r[molten.evidence.basalt_ucan.authority_receipt_binding] Pass-evidence gates MUST require Basalt/UCAN authority receipts for Basalt-governed capability admission, and those receipts MUST bind request ref, contract id, resource, ability, Basalt policy/source/export refs, UCAN proofset ref, UCAN verification receipt refs, derived grant refs, Basalt enforcement result, decision, diagnostics, and evidence-only caveats.

#### Scenario: Gate receipt includes Basalt/UCAN authority refs
- GIVEN a deterministic report whose side effects were admitted through Basalt/UCAN authority
- WHEN pass-evidence gate checking emits a gate receipt
- THEN the receipt includes artifact refs for the UCAN proofset, UCAN verification receipts, Basalt enforcement receipt, derived grant refs, and request ref
- AND each ref is recomputed or validated before the gate passes.

#### Scenario: Tampered authority binding fails gate
- GIVEN a report whose Basalt/UCAN authority receipt names a stale proofset ref, derived grant ref, Basalt policy ref, enforcement receipt, or request ref
- WHEN pass-evidence gate checking runs
- THEN the gate fails closed before accepting the report as pass evidence.

### Requirement: Authority receipt replay remains evidence-only
r[molten.evidence.basalt_ucan.replay_evidence_only] Basalt/UCAN authority receipts used during replay MUST validate historical decisions without minting new current authority, bypassing current revocation/replay checks, or replacing subsystem-specific gates.

#### Scenario: Replay validates old decision without current authority
- GIVEN a historical report with a passing Basalt/UCAN authority receipt
- AND the underlying token is now expired or revoked
- WHEN deterministic replay validates the historical report
- THEN replay may accept the historical receipt as recorded evidence
- AND a new current operation with the same token must still deny unless current UCAN/Basalt admission passes.
