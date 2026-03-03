## MODIFIED Requirements

### Requirement: NAT Traversal and Relay

The system SHALL use iroh's built-in NAT traversal (STUN/TURN-like) and relay servers to establish connectivity between nodes behind NATs. When the `relay-server` feature is enabled, the system SHALL prefer the cluster's own relay servers over external infrastructure.

#### Scenario: Direct connection via hole-punching

- **WHEN** two nodes behind different NATs attempt to connect
- **THEN** iroh SHALL attempt direct UDP hole-punching first

#### Scenario: Relay fallback

- **GIVEN** direct connectivity cannot be established
- **WHEN** hole-punching fails
- **THEN** traffic SHALL fall back to an iroh relay server
- **AND** the connection SHALL be functionally equivalent to a direct one

#### Scenario: Cluster relay preferred over external

- **GIVEN** the `relay-server` feature is enabled and cluster relays are available
- **WHEN** a node configures its relay mode
- **THEN** the node SHALL use `RelayMode::Custom` with the cluster's relay URLs
- **AND** external relay infrastructure (n0 public relays) SHALL NOT be used unless explicitly configured

#### Scenario: Graceful degradation without cluster relays

- **GIVEN** the `relay-server` feature is not enabled
- **WHEN** a node configures its relay mode
- **THEN** the node SHALL use its configured relay mode as before (default, custom, or disabled)
- **AND** existing behavior SHALL be unchanged

### Requirement: Cluster Bootstrap and Discovery

The system SHALL support multiple discovery mechanisms for cluster formation: direct endpoint IDs, gossip-based discovery, optional BitTorrent DHT (`global-discovery` feature), and cluster relay URLs (`relay-server` feature).

#### Scenario: Bootstrap via known peers

- **GIVEN** a new node with a list of known endpoint IDs
- **WHEN** the node starts
- **THEN** it SHALL connect to the known peers and join the cluster

#### Scenario: Gossip-based peer discovery

- **GIVEN** a node connected to at least one cluster member
- **WHEN** new nodes join the cluster
- **THEN** their endpoint information SHALL propagate via gossip
- **AND** all nodes SHALL eventually discover the new member

#### Scenario: Relay URL discovery via membership

- **GIVEN** a node that has joined the Raft cluster
- **WHEN** the node reads the cluster membership
- **THEN** it SHALL discover relay URLs for all nodes that have them in their metadata
- **AND** it SHALL update its local relay map accordingly
