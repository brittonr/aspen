## ADDED Requirements

### Requirement: Length-prefix framing on forwarded RPC responses

Forwarded RPC responses between cluster nodes MUST use a 4-byte big-endian length prefix before the serialized payload. The receiver MUST validate the declared length before reading the payload.

#### Scenario: Normal forwarded response

- **WHEN** the protocol handler sends a response to a forwarding node
- **THEN** the response is prefixed with a 4-byte BE u32 containing the payload length
- **AND** the receiver reads the length, validates it ≤ MAX_CLIENT_MESSAGE_SIZE, then reads exactly that many bytes

#### Scenario: Truncated stream

- **WHEN** a QUIC stream is reset or closed before the full payload is delivered
- **THEN** the receiver detects incomplete data (read fewer bytes than declared length)
- **AND** returns a framing error (not a deserialization error)
- **AND** the caller retries via the existing reconnect logic

#### Scenario: Invalid length prefix

- **WHEN** the declared length exceeds MAX_CLIENT_MESSAGE_SIZE or the stream delivers fewer than 4 bytes for the header
- **THEN** the receiver returns an error and evicts the cached connection
