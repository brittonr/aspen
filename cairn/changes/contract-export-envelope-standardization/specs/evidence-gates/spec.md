# Evidence Gates Delta: Contract export envelope standardization

### Requirement: Nickel contract exports carry metadata envelopes
r[molten.evidence.contract_exports.metadata_envelope] Nickel-authored contract and fixture exports that are promoted to evidence MUST carry explicit schema id, schema version, source language, stable export identity, and payload fields before their content refs are bound into receipts.

#### Scenario: Envelope identifies export shape
- GIVEN a Nickel-authored contract export that will be checked in or bound into a receipt
- WHEN a reviewer inspects the exported value
- THEN the value names its schema id, schema version, source language, stable identity, and payload without relying on filename inference

#### Scenario: Missing metadata fails evidence promotion
- GIVEN a Nickel-authored contract export that omits schema id, schema version, source language, or stable identity metadata
- WHEN evidence promotion validation runs
- THEN promotion fails before receipts can treat the export as reviewed contract evidence

### Requirement: Contract export envelopes are evidence shape only
r[molten.evidence.contract_exports.evidence_only_metadata] Contract export metadata MUST identify evidence shape and reviewed source only, and MUST NOT grant authority, policy permission, source-gate freshness, adapter readiness, resource trust, transport admission, or plugin hostcall authority.

#### Scenario: Metadata does not grant authority
- GIVEN a contract export with a valid metadata envelope
- WHEN a subsystem needs runtime authority or freshness evidence
- THEN it still requires the subsystem-specific receipts and does not treat the envelope metadata as authority
