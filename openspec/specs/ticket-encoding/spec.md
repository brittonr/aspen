# Ticket Encoding Specification

## Purpose

Aspen ticket encoders and decoders preserve authority-bearing payload meaning by surfacing encoding failures explicitly and by keeping decode failures attributable to malformed, expired, or tampered input.
## Requirements
### Requirement: Ticket construction surfaces encoding failures

When a shareable ticket or ticket wrapper depends on fallible inner encoding, the constructor or serializer SHALL return an explicit error instead of substituting empty bytes or other default payloads.

#### Scenario: Oversized capability token blocks automerge sync ticket creation

- **GIVEN** an `AutomergeSyncTicket` wraps a `CapabilityToken` whose encoded form exceeds `MAX_TOKEN_SIZE`
- **WHEN** the ticket is created or serialized
- **THEN** the operation SHALL fail with an encoding error
- **AND** no ticket string SHALL be produced

#### Scenario: Postcard serialization error is surfaced

- **GIVEN** a ticket wrapper hits a postcard serialization failure while building its shareable payload
- **WHEN** the serializer runs
- **THEN** the caller SHALL receive an explicit error at the serialization call site
- **AND** the system SHALL NOT replace the payload with `Vec::new()` or other default bytes

### Requirement: Externally shared ticket payloads never use silent default fallbacks

Authority-bearing ticket and signed-identity encoders SHALL either serialize successfully or fail explicitly. They SHALL NOT emit empty or default payload bytes on serialization failure.

#### Scenario: Client ticket encoder does not emit empty payload

- **WHEN** an `AspenClientTicket` is serialized for sharing with a user or service
- **THEN** the encoder SHALL either return the correct serialized payload or fail explicitly
- **AND** it SHALL NOT mint a ticket string from an empty payload

#### Scenario: Signed cluster ticket encoder fails loudly on impossible serializer bug

- **WHEN** the signed cluster ticket encoder encounters a serialization failure in a trait-imposed `to_bytes() -> Vec<u8>` path
- **THEN** the process SHALL fail loudly with contextual diagnostics
- **AND** it SHALL NOT continue with an empty payload that changes the meaning of the ticket

### Requirement: Decode failures remain attributable to malformed input, not hidden encode fallback

Receivers SHALL only see decode errors for genuinely malformed or tampered ticket payloads, not because the sender silently replaced a failed serialization with empty bytes.

#### Scenario: Invalid ticket string is still rejected

- **WHEN** a caller provides a malformed ticket string or corrupted payload bytes
- **THEN** deserialization SHALL fail as it does today
- **AND** the failure SHALL reflect invalid input rather than a sender-side silent fallback path

### Requirement: Signed cluster ticket encoders never use silent default fallbacks

Signed cluster ticket encoders SHALL either serialize successfully or fail loudly. They SHALL NOT emit empty or default payload bytes on serialization failure.

ID: ticket.encoding.signed-cluster-ticket-encoders-never-use-silent-default-fallbacks
#### Scenario: Signed cluster ticket encoder fails loudly on impossible serializer bug
ID: ticket.encoding.signed-cluster-ticket-encoders-never-use-silent-default-fallbacks.signed-cluster-ticket-encoder-fails-loudly-on-impossible-serializer-bug

- **WHEN** the signed cluster ticket encoder encounters a serialization failure in a trait-imposed `to_bytes() -> Vec<u8>` path
- **THEN** it SHALL trigger a deterministic `expect(...)` / panic invariant break with contextual diagnostics from that encoder path
- **AND** it SHALL NOT continue with an empty payload that changes the meaning of the signed ticket

### Requirement: Signed cluster ticket decode failures remain attributable to malformed input

Receivers SHALL only see signed-ticket decode failures for genuinely malformed, expired, or tampered payloads, not because the sender silently replaced a failed serialization with empty bytes.

ID: ticket.encoding.signed-cluster-ticket-decode-failures-remain-attributable-to-malformed-input
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

### Requirement: Hook ticket encoder never substitutes an empty payload

`AspenHookTicket` encoding SHALL either produce the correct serialized payload or fail loudly on impossible serializer bugs. It SHALL NOT replace a failed encoding with `Vec::new()` or any other default payload bytes.

ID: ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload
#### Scenario: Hook ticket encoder fails loudly on serializer invariant break
ID: ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fails-loudly-on-serializer-invariant-break

- **GIVEN** `AspenHookTicket` uses a trait-imposed `to_bytes() -> Vec<u8>` path
- **WHEN** postcard serialization hits an impossible invariant break while encoding a bounded ticket payload
- **THEN** the process SHALL fail loudly with contextual diagnostics
- **AND** it SHALL NOT continue by minting an empty or default payload ticket

#### Scenario: Hook ticket encoder fail-loud proof is reviewable
ID: ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fail-loud-proof-is-reviewable

- **GIVEN** the hook ticket encoder fallback removal is complete
- **WHEN** the change is prepared for review
- **THEN** `openspec/changes/alloc-safe-hooks-ticket/evidence/final-validation.md` SHALL include a deterministic source-audit command proving `impl Ticket for AspenHookTicket` uses an `expect` fail-loud path and no `Vec::new()` fallback
- **AND** `openspec/changes/alloc-safe-hooks-ticket/evidence/implementation-diff.txt` SHALL show the fallback removal in the reviewed diff
- **AND** `openspec/changes/alloc-safe-hooks-ticket/verification.md` SHALL map the encoder requirement to those artifacts

