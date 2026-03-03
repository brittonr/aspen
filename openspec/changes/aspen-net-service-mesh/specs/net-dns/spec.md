## ADDED Requirements

### Requirement: MagicDNS Stub Resolver

The daemon SHALL run a local DNS server that resolves `*.aspen` queries by looking up the service registry.

#### Scenario: Resolve a known service

- **GIVEN** a service `mydb` is registered in the service registry
- **WHEN** a DNS A query for `mydb.aspen` is received
- **THEN** the resolver SHALL return a synthetic loopback address (e.g., `127.0.0.2`)
- **AND** the TTL SHALL be `NET_DNS_TTL_SECS` (5 seconds)

#### Scenario: Resolve an unknown service

- **WHEN** a DNS A query for `unknown.aspen` is received
- **AND** no service named `unknown` is registered
- **THEN** the resolver SHALL return NXDOMAIN

#### Scenario: Non-.aspen query forwarded

- **WHEN** a DNS query for `example.com` is received
- **THEN** the resolver SHALL forward the query to the system's upstream DNS resolver
- **AND** return the upstream response to the client

#### Scenario: AAAA query for .aspen

- **WHEN** a DNS AAAA query for `mydb.aspen` is received
- **THEN** the resolver SHALL return an empty response (no AAAA record)
- **AND** the response SHALL have NOERROR status (not NXDOMAIN)

### Requirement: Loopback Address Management

The DNS resolver SHALL manage a mapping of service names to loopback addresses.

#### Scenario: Consistent address assignment

- **GIVEN** `mydb.aspen` resolves to `127.0.0.2`
- **WHEN** the same query is made again
- **THEN** the resolver SHALL return the same address `127.0.0.2`

#### Scenario: Address recycling

- **WHEN** the number of active service-to-address mappings reaches `MAX_NET_DNS_LOOPBACK_ADDRS` (254)
- **AND** a new service needs an address
- **THEN** the least-recently-used mapping SHALL be evicted
- **AND** its address SHALL be reassigned

### Requirement: DNS Custom Overrides

Users SHALL be able to set custom DNS records for `.aspen` names.

#### Scenario: Custom A record

- **WHEN** a user sets a DNS override: `myhost.aspen` → `10.0.1.5`
- **THEN** DNS queries for `myhost.aspen` SHALL return `10.0.1.5`
- **AND** the override SHALL be stored at `/_sys/net/dns/myhost`

#### Scenario: Override takes precedence

- **GIVEN** a service `mydb` is registered AND a DNS override `mydb.aspen` → `10.0.1.5` exists
- **WHEN** a DNS query for `mydb.aspen` is received
- **THEN** the override SHALL take precedence, returning `10.0.1.5`

### Requirement: DNS Server Configuration

#### Scenario: Default port

- **WHEN** the DNS resolver starts without explicit port configuration
- **THEN** it SHALL bind on UDP port `5353`

#### Scenario: Custom port

- **WHEN** the DNS resolver is configured with port `53`
- **THEN** it SHALL bind on UDP port `53`
- **AND** the user is responsible for having appropriate permissions (CAP_NET_BIND_SERVICE)

#### Scenario: DNS resolver is optional

- **WHEN** the daemon starts with `--no-dns` flag
- **THEN** the DNS resolver SHALL NOT be started
- **AND** the SOCKS5 proxy SHALL still function normally
