## ADDED Requirements

### Requirement: Discovery trait implementation

`ClusterDiscovery` implements `iroh::Discovery` with file-based persistence.

#### Scenario: Publish on address change

- **WHEN** iroh calls `publish()` with new direct addresses
- **THEN** the node writes `{ node_id, relay_url, direct_addresses }` to `<data_dir>/discovery/<public_key_hex>.json` atomically (write-tmp + rename)

#### Scenario: Resolve returns persisted address

- **WHEN** iroh calls `resolve(peer_node_id)` for a known peer
- **THEN** `ClusterDiscovery` reads the peer's discovery file from sibling data directories and returns the addresses as a `DiscoveryItem`

#### Scenario: Resolve falls back to Raft membership

- **WHEN** no discovery file exists for the peer
- **THEN** `ClusterDiscovery` returns addresses from the Raft membership entry for that node_id

### Requirement: Startup address seeding

Nodes pre-populate iroh's address book on startup.

#### Scenario: Seed from discovery files

- **WHEN** a node starts and sibling discovery files exist with fresh addresses
- **THEN** the node calls `endpoint.add_node_addr()` for each peer, giving iroh the addresses to try

#### Scenario: Seed from Raft membership

- **WHEN** no discovery files exist but Raft membership has peer addresses
- **THEN** the node calls `endpoint.add_node_addr()` with the membership addresses (may be stale but gives iroh starting points)

### Requirement: Restart recovery

Three-node cluster recovers quorum after simultaneous restart without relay.

#### Scenario: Air-gapped restart recovery

- **WHEN** 3 nodes on the same machine restart simultaneously with new iroh ports, relay disabled, and mDNS disabled
- **THEN** within 10 seconds, all nodes discover each other's new addresses via `ClusterDiscovery` and restore Raft quorum

### Requirement: Shutdown persistence

Clean shutdown flushes current addresses to disk.

#### Scenario: Graceful shutdown saves addresses

- **WHEN** a node shuts down gracefully
- **THEN** `ClusterDiscovery::publish()` is called with current addresses, and the factory's `peer_addrs.json` is flushed

### Requirement: Additive discovery

`ClusterDiscovery` works alongside existing discovery mechanisms.

#### Scenario: Coexists with mDNS and DNS

- **WHEN** mDNS and DNS discovery are also enabled
- **THEN** `ClusterDiscovery` is added via `.add_discovery()` and iroh queries all discovery services, using whichever resolves first
