## ADDED Requirements

### Requirement: Signed cluster ticket encoders never use silent default fallbacks
ID: ticket.encoding.signed-cluster-ticket-encoders-never-use-silent-default-fallbacks

Signed cluster ticket encoders SHALL either serialize successfully or fail loudly. They SHALL NOT emit empty or default payload bytes on serialization failure.

#### Scenario: Signed cluster ticket encoder fails loudly on impossible serializer bug
ID: ticket.encoding.signed-cluster-ticket-encoders-never-use-silent-default-fallbacks.signed-cluster-ticket-encoder-fails-loudly-on-impossible-serializer-bug

- **WHEN** the signed cluster ticket encoder encounters a serialization failure in a trait-imposed `to_bytes() -> Vec<u8>` path
- **THEN** it SHALL trigger a deterministic `expect(...)` / panic invariant break with contextual diagnostics from that encoder path
- **AND** it SHALL NOT continue with an empty payload that changes the meaning of the signed ticket

### Requirement: Signed cluster ticket decode failures remain attributable to malformed input
ID: ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input

Receivers SHALL only see signed-ticket decode failures for genuinely malformed, expired, or tampered payloads, not because the sender silently replaced a failed serialization with empty bytes.

#### Scenario: Invalid signed cluster ticket string is still rejected
ID: ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input.invalid-signed-cluster-ticket-string-is-still-rejected

- **WHEN** a caller provides a malformed signed cluster ticket string or corrupted payload bytes
- **THEN** deserialization or verification SHALL fail
- **AND** the failure SHALL reflect invalid input rather than a sender-side empty-payload fallback path

#### Scenario: Signed cluster ticket encoding proof is reviewable
ID: ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input.signed-cluster-ticket-encoding-proof-is-reviewable

- **GIVEN** the signed encoder fallback removal and malformed-input rejection contract
- **WHEN** the seam is prepared for review
- **THEN** saved artifacts under `openspec/changes/alloc-safe-cluster-ticket/evidence/` SHALL include targeted signed malformed-input rejection tests plus a deterministic source audit proving `SignedAspenClusterTicket::to_bytes()` no longer uses empty-payload fallback
- **AND** `openspec/changes/alloc-safe-cluster-ticket/verification.md` SHALL map those ticket-encoding proofs to the checked tasks
