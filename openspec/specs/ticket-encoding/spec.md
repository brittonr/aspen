## ADDED Requirements

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
