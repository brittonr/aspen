## MODIFIED Requirements

### Requirement: Net Capability Variants

The `aspen-auth` Capability enum SHALL be extended with variants for service mesh authorization.

#### Scenario: NetConnect capability authorizes tunnel creation

- **GIVEN** a capability token with `NetConnect { service_prefix: "prod/" }`
- **WHEN** the daemon attempts to connect to service `prod/mydb` on port `5432`
- **THEN** `token.authorizes(Operation::NetConnect { service: "prod/mydb", port: 5432 })` SHALL return `true`

#### Scenario: NetConnect capability denies out-of-scope service

- **GIVEN** a capability token with `NetConnect { service_prefix: "prod/" }`
- **WHEN** the daemon attempts to connect to service `staging/mydb`
- **THEN** `token.authorizes(Operation::NetConnect { service: "staging/mydb", port: 5432 })` SHALL return `false`

#### Scenario: NetConnect wildcard

- **GIVEN** a capability token with `NetConnect { service_prefix: "" }` (empty prefix = all services)
- **WHEN** the daemon attempts to connect to any service
- **THEN** authorization SHALL succeed

#### Scenario: NetPublish capability authorizes service registration

- **GIVEN** a capability token with `NetPublish { service_prefix: "myapp-" }`
- **WHEN** the daemon attempts to publish service `myapp-web`
- **THEN** `token.authorizes(Operation::NetPublish { service: "myapp-web" })` SHALL return `true`

#### Scenario: NetPublish capability denies out-of-scope name

- **GIVEN** a capability token with `NetPublish { service_prefix: "myapp-" }`
- **WHEN** the daemon attempts to publish service `other-web`
- **THEN** authorization SHALL fail

#### Scenario: NetAdmin capability authorizes all net operations

- **GIVEN** a capability token with `NetAdmin`
- **WHEN** the daemon attempts any net operation (connect, publish, unpublish, DNS override)
- **THEN** authorization SHALL succeed

### Requirement: Capability Containment for Delegation

Delegation SHALL enforce that child capabilities are subsets of parent capabilities.

#### Scenario: NetConnect delegation with narrower prefix

- **GIVEN** a parent token with `NetConnect { service_prefix: "prod/" }`
- **WHEN** delegating a child token with `NetConnect { service_prefix: "prod/db-" }`
- **THEN** the delegation SHALL succeed (child is narrower)

#### Scenario: NetConnect delegation with wider prefix rejected

- **GIVEN** a parent token with `NetConnect { service_prefix: "prod/" }`
- **WHEN** delegating a child token with `NetConnect { service_prefix: "" }` (all services)
- **THEN** the delegation SHALL fail (child is wider than parent)

#### Scenario: NetAdmin contains all net capabilities

- **GIVEN** a parent token with `NetAdmin`
- **WHEN** delegating a child token with any `NetConnect` or `NetPublish` variant
- **THEN** the delegation SHALL succeed

#### Scenario: NetConnect does not contain NetPublish

- **GIVEN** a parent token with `NetConnect { service_prefix: "" }`
- **WHEN** delegating a child token with `NetPublish { service_prefix: "" }`
- **THEN** the delegation SHALL fail (different capability type)

### Requirement: Token-Based Daemon Authorization

The daemon SHALL hold a capability token and verify it locally before every net operation.

#### Scenario: Daemon startup with token

- **WHEN** the daemon starts with `--token <base64-encoded-token>`
- **THEN** the token SHALL be decoded and verified (signature, expiration, revocation)
- **AND** the token SHALL be stored for use during the daemon's lifetime

#### Scenario: SOCKS5 connect checks token

- **WHEN** a SOCKS5 CONNECT request arrives for service `mydb`
- **THEN** the daemon SHALL call `verifier.authorize(token, Operation::NetConnect { service: "mydb", port })` before creating the tunnel
- **AND** if authorization fails, return SOCKS5 connection refused error (0x05)

#### Scenario: Port forward checks token

- **WHEN** `aspen net forward 5432:mydb` is invoked with `--token <token>`
- **THEN** the token SHALL be verified and `NetConnect` capability checked before binding the listener
- **AND** if authorization fails, the command SHALL exit with a clear error

#### Scenario: Service publish checks token

- **WHEN** the daemon publishes a service
- **THEN** the daemon SHALL verify `NetPublish` capability for the service name
- **AND** if authorization fails, the publish SHALL fail with a clear error

#### Scenario: Token expiration during daemon lifetime

- **WHEN** the daemon's token expires while running
- **THEN** new SOCKS5 connections and publish operations SHALL fail with "token expired"
- **AND** existing active tunnels SHALL NOT be interrupted (they were authorized at connect time)
- **AND** the daemon SHALL log a warning about the expired token

### Requirement: Token Verification Is Local

#### Scenario: No Raft read for authorization

- **WHEN** a SOCKS5 CONNECT request is authorized
- **THEN** the token verification SHALL NOT require any network I/O or Raft KV read
- **AND** verification SHALL use only the token's cryptographic signature and the verifier's trusted roots

#### Scenario: Revocation check is periodic

- **WHEN** the daemon refreshes its revocation list
- **THEN** it SHALL poll the Raft-backed revocation store every `NET_REVOCATION_POLL_INTERVAL_SECS` (60 seconds)
- **AND** between polls, the last-known revocation set SHALL be used for verification
