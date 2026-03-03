## ADDED Requirements

### Requirement: TCP Port Forwarding

Users SHALL be able to forward a local TCP port to a named remote service through iroh.

#### Scenario: Forward local port to named service

- **GIVEN** a service `mydb` is registered with endpoint ID `abc123...` and port `5432`
- **WHEN** a user runs `aspen net forward 5432:mydb`
- **THEN** a TCP listener SHALL bind on `localhost:5432`
- **AND** every incoming TCP connection SHALL be tunneled to the remote endpoint's port `5432`
- **AND** tunneling SHALL use `DownstreamProxy::forward_tcp_listener()` with `ProxyMode::Tcp`

#### Scenario: Forward with explicit remote port

- **WHEN** a user runs `aspen net forward 8080:web:9090`
- **THEN** local port `8080` SHALL forward to the `web` service's endpoint on port `9090`
- **AND** the registered port in the service entry SHALL be ignored

#### Scenario: Forward with default remote port

- **GIVEN** a service `mydb` registered with port `5432`
- **WHEN** a user runs `aspen net forward 5432:mydb`
- **THEN** the remote port SHALL default to the service's registered port (`5432`)

#### Scenario: Forward to non-existent service

- **WHEN** a user runs `aspen net forward 5432:unknown`
- **AND** no service named `unknown` exists
- **THEN** the command SHALL fail with a clear error: "service 'unknown' not found"

#### Scenario: Forward denied by capability token

- **GIVEN** a token that does not grant `NetConnect` capability for service `mydb`
- **WHEN** a user runs `aspen net forward 5432:mydb --token <token>`
- **THEN** the command SHALL fail with an authorization error: "token does not authorize connecting to 'mydb'"

#### Scenario: Local port already in use

- **WHEN** a user runs `aspen net forward 5432:mydb` and port `5432` is already bound
- **THEN** the command SHALL fail with a clear error indicating the port is in use

### Requirement: Forward Lifecycle

#### Scenario: Graceful shutdown

- **WHEN** a port forward receives a shutdown signal (Ctrl+C or SIGTERM)
- **THEN** active connections SHALL be drained with a bounded timeout
- **AND** the TCP listener SHALL be closed
- **AND** QUIC connections SHALL be released to the connection pool

#### Scenario: Remote peer disconnects

- **WHEN** the remote peer's iroh endpoint becomes unreachable during an active tunnel
- **THEN** the corresponding local TCP connection SHALL be closed
- **AND** other forwarded connections SHALL not be affected

### Requirement: Forward bind address

#### Scenario: Default bind to localhost

- **WHEN** a user runs `aspen net forward 5432:mydb` without specifying a bind address
- **THEN** the listener SHALL bind on `127.0.0.1:5432` (localhost only)

#### Scenario: Custom bind address

- **WHEN** a user runs `aspen net forward 0.0.0.0:5432:mydb`
- **THEN** the listener SHALL bind on `0.0.0.0:5432` (all interfaces)
