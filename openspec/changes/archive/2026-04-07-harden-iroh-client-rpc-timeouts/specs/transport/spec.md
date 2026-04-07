## ADDED Requirements

### Requirement: Client RPC exchanges are fully bounded

The system SHALL bound every blocking stage of a client-initiated QUIC RPC exchange. After a connection has been acquired, `open_bi()`, request-body writes, stream finish, and response reads SHALL each execute under an explicit timeout policy. No stage SHALL be left unbounded.

#### Scenario: Stream open stalls after connection success

- **GIVEN** a client has acquired a QUIC connection to a peer
- **WHEN** opening a bidirectional stream does not complete before the configured timeout budget
- **THEN** the RPC SHALL fail with a timeout error
- **AND** the caller SHALL regain control without waiting indefinitely

#### Scenario: Request flush stalls under flow control

- **GIVEN** a client has opened a bidirectional stream
- **WHEN** `write_all()` or `finish()` does not complete because the peer stops draining request bytes
- **THEN** the RPC SHALL fail with a timeout error
- **AND** the connection SHALL NOT be reused from any cache that borrowed it for the failed request

#### Scenario: Response body never arrives

- **GIVEN** a client has successfully flushed a request
- **WHEN** the peer does not deliver a complete response before the configured timeout budget
- **THEN** the RPC SHALL fail with a response timeout error

### Requirement: Timeout coverage is consistent across client entrypoints

The timeout policy for post-connect QUIC RPC exchange stages SHALL be applied consistently to the shared client entrypoints used by application clients, the CLI, federation sync clients, and remote-cluster proxy forwarding.

#### Scenario: CLI request uses bounded stream exchange

- **WHEN** the CLI sends a client RPC over a cached or freshly established QUIC connection
- **THEN** stream open, request write, stream finish, and response read SHALL all be bounded by explicit timeouts

#### Scenario: Federation sync request uses bounded stream exchange

- **WHEN** the federation sync client performs handshake, resource listing, state lookup, object sync, or push over QUIC
- **THEN** each request SHALL use the same bounded stream-exchange policy

#### Scenario: Proxy forwarding uses bounded stream exchange

- **WHEN** the proxy forwards a client RPC to a remote cluster over QUIC
- **THEN** the forwarding request SHALL bound stream open, request write, stream finish, and response read

#### Scenario: CLI blob fetch uses bounded stream exchange

- **WHEN** the CLI fetches a blob via `send_get_blob()` over a QUIC connection
- **THEN** stream open, request write, stream finish, and response read SHALL all be bounded by explicit timeouts
- **AND** the response read timeout SHALL accommodate large payloads by using a dedicated read budget rather than the default short RPC budget
