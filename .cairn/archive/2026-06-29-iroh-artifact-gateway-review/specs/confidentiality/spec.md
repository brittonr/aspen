## ADDED Requirements

### Requirement: Operator gateway visibility and retention gates
r[molten.operator_gateway.visibility_retention] Molten MUST apply confidentiality, redaction, reveal-authority, retention, and visibility policy checks before an operator gateway exposes object names, refs, MIME hints, sizes, collection membership, or bytes.

#### Scenario: Protected object denies without reveal evidence
- GIVEN an operator gateway request for a protected-commitment or otherwise confidential object
- WHEN reveal authority or required policy evidence is missing
- THEN Molten emits a deny receipt before rendering names, refs, MIME hints, sizes, or bytes
- AND diagnostics avoid leaking the protected plaintext ref.

#### Scenario: Public profile redacts sensitive metadata
- GIVEN a gateway index request under a public or diagnostic profile
- WHEN bundle members include sensitive names or refs
- THEN Molten renders only policy-admitted redacted metadata
- AND the gateway index receipt records that redaction occurred.
