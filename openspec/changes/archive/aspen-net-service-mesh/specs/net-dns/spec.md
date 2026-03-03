## MODIFIED Requirements

### Requirement: DNS Record Auto-Creation on Service Publish

When a service is published to the mesh, DNS records SHALL be automatically created in the `aspen` zone via the existing `aspen-dns` crate's `DnsStore`.

#### Scenario: Publish creates SRV and A records

- **GIVEN** a running Aspen cluster with `aspen-dns` enabled
- **WHEN** a service `mydb` is published with endpoint ID `abc123...` and port `5432`
- **THEN** the following DNS records SHALL be created via `DnsStore::set_record()`:
  - `SRV _tcp.mydb.aspen` → `0 0 5432 {endpoint_short}.aspen`
  - `A mydb.aspen` → synthetic loopback address (e.g., `127.0.0.2`)
- **AND** the records SHALL have TTL equal to `NET_DNS_TTL_SECS` (30 seconds)

#### Scenario: Unpublish removes DNS records

- **GIVEN** a service `mydb` has DNS records in the `aspen` zone
- **WHEN** the service is unpublished
- **THEN** the SRV and A records for `mydb.aspen` SHALL be deleted via `DnsStore::delete_record()`

#### Scenario: DNS records sync via iroh-docs

- **WHEN** a DNS record is created on the server side
- **THEN** the record SHALL propagate to all `AspenDnsClient` instances via iroh-docs P2P CRDT sync
- **AND** the `DnsProtocolServer` on each client SHALL serve the updated record

### Requirement: DnsProtocolServer Serves aspen Zone

The existing `DnsProtocolServer` from `aspen-dns` SHALL serve the `aspen` zone for service mesh name resolution.

#### Scenario: Resolve a published service via DNS

- **GIVEN** a service `mydb` is published and DNS records exist
- **AND** a `DnsProtocolServer` is running on port 5353 with zone `aspen`
- **WHEN** a DNS A query for `mydb.aspen` is received
- **THEN** the server SHALL return the synthetic A record from the local cache

#### Scenario: SRV query for service discovery

- **GIVEN** a service `mydb` is published on port 5432
- **WHEN** a DNS SRV query for `_tcp.mydb.aspen` is received
- **THEN** the server SHALL return the SRV record with the port and target

#### Scenario: NXDOMAIN for unknown service

- **WHEN** a DNS query for `unknown.aspen` is received
- **AND** no service named `unknown` is published
- **THEN** the server SHALL return NXDOMAIN

#### Scenario: Non-aspen queries forwarded

- **WHEN** a DNS query for `example.com` is received
- **AND** forwarding is enabled with upstream resolvers configured
- **THEN** the query SHALL be forwarded to the configured upstream DNS server

### Requirement: AspenDnsClient Integration in Daemon

The service mesh daemon SHALL use `AspenDnsClient` for local DNS cache and resolution.

#### Scenario: Daemon starts with DNS client

- **WHEN** the daemon starts with `--ticket <cluster-ticket>`
- **THEN** it SHALL create an `AspenDnsClient` that syncs the `aspen` zone via iroh-docs
- **AND** start a `DnsProtocolServer` serving the synced zone on the configured port

#### Scenario: DNS is optional

- **WHEN** the daemon starts with `--no-dns` flag
- **THEN** no `DnsProtocolServer` SHALL be started
- **AND** name resolution SHALL still work via the service registry (RPC-based)

### Requirement: Loopback Address Management

The service registry SHALL manage a mapping of service names to loopback addresses for DNS A records.

#### Scenario: Consistent address assignment

- **GIVEN** `mydb.aspen` is assigned loopback `127.0.0.2`
- **WHEN** the A record is queried multiple times
- **THEN** the same address `127.0.0.2` SHALL be returned

#### Scenario: Address recycling

- **WHEN** the number of active service-to-address mappings reaches `MAX_NET_DNS_LOOPBACK_ADDRS` (254)
- **AND** a new service needs an address
- **THEN** the least-recently-used mapping SHALL be evicted
