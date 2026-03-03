## ADDED Requirements

### Requirement: SOCKS5 Server

The daemon SHALL run a SOCKS5 proxy server (RFC 1928) that accepts TCP connections and tunnels them to named services through iroh.

#### Scenario: SOCKS5 CONNECT to named service

- **GIVEN** a service `mydb` is registered with endpoint ID `abc123...` and port `5432`
- **AND** the SOCKS5 proxy is running on port `1080`
- **WHEN** a client sends a SOCKS5 CONNECT request for `mydb.aspen:5432`
- **THEN** the proxy SHALL resolve `mydb` from the service registry
- **AND** create a CONNECT tunnel via `DownstreamProxy::create_tunnel()` to the resolved endpoint
- **AND** return SOCKS5 success response (0x00)
- **AND** bidirectionally forward bytes between the client TCP stream and the QUIC tunnel

#### Scenario: SOCKS5 CONNECT with explicit port override

- **GIVEN** a service `web` is registered with port `8080`
- **WHEN** a client sends a SOCKS5 CONNECT request for `web.aspen:9090`
- **THEN** the proxy SHALL use port `9090` (the client-specified port), not the registered port `8080`

#### Scenario: SOCKS5 CONNECT to non-existent service

- **WHEN** a client sends a SOCKS5 CONNECT request for `unknown.aspen:5432`
- **AND** no service named `unknown` exists in the registry
- **THEN** the proxy SHALL return SOCKS5 host unreachable error (0x04)

#### Scenario: SOCKS5 CONNECT denied by capability token

- **GIVEN** the daemon holds a token with `NetConnect { service_prefix: "prod/" }`
- **WHEN** a client sends a SOCKS5 CONNECT request for `staging-db.aspen:5432`
- **THEN** the proxy SHALL verify the token does not authorize `NetConnect { service: "staging-db" }`
- **AND** return SOCKS5 connection refused error (0x05)

#### Scenario: SOCKS5 CONNECT to non-.aspen host

- **WHEN** a client sends a SOCKS5 CONNECT request for `example.com:443`
- **THEN** the proxy SHALL reject the request with SOCKS5 connection refused error (0x05)
- **AND** log that non-.aspen destinations are not supported

### Requirement: SOCKS5 Protocol Compliance

The SOCKS5 implementation SHALL comply with RFC 1928 for the supported subset.

#### Scenario: Authentication negotiation

- **WHEN** a client sends a SOCKS5 greeting with supported authentication methods
- **THEN** the proxy SHALL respond with "no authentication required" (method 0x00)
- **AND** the proxy SHALL NOT support username/password or GSSAPI authentication

#### Scenario: CONNECT command only

- **WHEN** a client sends a SOCKS5 BIND or UDP ASSOCIATE command
- **THEN** the proxy SHALL return "command not supported" error (0x07)

#### Scenario: Domain name address type

- **WHEN** a client sends a CONNECT with address type 0x03 (domain name)
- **THEN** the proxy SHALL parse the domain name and resolve it via the service registry

#### Scenario: IPv4/IPv6 address type

- **WHEN** a client sends a CONNECT with address type 0x01 (IPv4) or 0x04 (IPv6)
- **THEN** the proxy SHALL reject the request
- **AND** the proxy SHALL only support domain name resolution for `.aspen` services

### Requirement: SOCKS5 Resource Bounds

#### Scenario: Maximum concurrent connections

- **WHEN** the number of active SOCKS5 connections reaches `MAX_SOCKS5_CONNECTIONS` (1,000)
- **THEN** new connections SHALL be rejected until existing ones close

#### Scenario: Connection timeout

- **WHEN** a SOCKS5 handshake is not completed within `SOCKS5_HANDSHAKE_TIMEOUT_SECS` (10 seconds)
- **THEN** the connection SHALL be closed

#### Scenario: Idle connection timeout

- **WHEN** a tunneled connection has no data transfer for `SOCKS5_IDLE_TIMEOUT_SECS` (300 seconds)
- **THEN** the connection SHALL be closed
