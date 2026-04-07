## ADDED Requirements

### Requirement: Auxiliary QUIC request/response clients are fully bounded

The system SHALL apply the same bounded post-connect QUIC exchange policy to the remaining auxiliary request/response clients that still open bidirectional streams directly. After a connection has been acquired, `open_bi()`, request-body writes, stream finish, and response reads SHALL each run under explicit timeout coverage.

#### Scenario: Git remote helper uses bounded stream exchange

- **WHEN** `git-remote-aspen` sends a client RPC over QUIC
- **THEN** stream open, request write, stream finish, and response read SHALL all be bounded

#### Scenario: TUI and hook clients use bounded stream exchange

- **WHEN** the TUI RPC client, hook client, or CLI hook forwarding path sends a QUIC request
- **THEN** the request SHALL use the same stage-bounded exchange policy as the main CLI client

#### Scenario: Storage bridge clients use bounded stream exchange

- **WHEN** the snix RPC service clients or blob replication adapters send QUIC requests
- **THEN** the request SHALL bound stream open, request write, stream finish, and response read
- **AND** large-response paths SHALL use a dedicated read budget when the default RPC budget is too short
