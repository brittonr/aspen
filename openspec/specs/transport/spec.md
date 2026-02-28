# Transport Specification

## Purpose

All network communication in Aspen uses iroh QUIC with ALPN-based protocol routing. There is no HTTP API. Iroh provides P2P connectivity, NAT traversal, relay fallback, and content-addressed blob transfer.

## Requirements

### Requirement: Iroh-Only Networking

The system SHALL use iroh QUIC for ALL network communication. No raw TCP, HTTP, or other transport protocols SHALL be used.

#### Scenario: Client-to-node communication

- GIVEN a client with the node's iroh endpoint ID
- WHEN the client sends an RPC request
- THEN the request SHALL be routed over iroh QUIC using `CLIENT_ALPN`

#### Scenario: Node-to-node communication

- GIVEN two nodes in the same cluster
- WHEN node A sends a Raft message to node B
- THEN the message SHALL be routed over iroh QUIC using `RAFT_ALPN`

### Requirement: ALPN Protocol Routing

The system SHALL use ALPN (Application-Layer Protocol Negotiation) to multiplex different protocols over a single iroh endpoint. Each protocol SHALL have a unique ALPN identifier.

#### Scenario: Raft protocol routing

- GIVEN an incoming QUIC connection with ALPN `RAFT_ALPN`
- WHEN the connection is accepted
- THEN it SHALL be routed to the Raft network handler

#### Scenario: Client API routing

- GIVEN an incoming connection with ALPN `CLIENT_ALPN`
- WHEN the connection is accepted
- THEN it SHALL be routed to the client protocol handler

#### Scenario: TUI protocol routing

- GIVEN an incoming connection with ALPN `TUI_ALPN`
- WHEN the connection is accepted
- THEN it SHALL be routed to the TUI real-time handler

#### Scenario: Gossip protocol routing

- GIVEN an incoming connection with ALPN `GOSSIP_ALPN`
- WHEN the connection is accepted
- THEN it SHALL be routed to the gossip protocol handler

### Requirement: NAT Traversal and Relay

The system SHALL use iroh's built-in NAT traversal (STUN/TURN-like) and relay servers to establish connectivity between nodes behind NATs.

#### Scenario: Direct connection via hole-punching

- GIVEN two nodes behind different NATs
- WHEN a connection is initiated
- THEN iroh SHALL attempt direct UDP hole-punching first

#### Scenario: Relay fallback

- GIVEN direct connectivity cannot be established
- WHEN hole-punching fails
- THEN traffic SHALL fall back to an iroh relay server
- AND the connection SHALL be functionally equivalent to a direct one

### Requirement: Connection Limits

The system SHALL enforce fixed connection limits to prevent resource exhaustion.

#### Scenario: Maximum peers

- GIVEN the node has `MAX_PEERS` (1,000) connected peers
- WHEN a new peer attempts to connect
- THEN the connection SHALL be rejected

#### Scenario: Connection timeouts

- GIVEN a connection attempt to a remote node
- WHEN 5 seconds elapse without establishing the connection
- THEN the attempt SHALL time out with an error

### Requirement: Cluster Bootstrap and Discovery

The system SHALL support multiple discovery mechanisms for cluster formation: direct endpoint IDs, gossip-based discovery, and optional BitTorrent DHT (`global-discovery` feature).

#### Scenario: Bootstrap via known peers

- GIVEN a new node with a list of known endpoint IDs
- WHEN the node starts
- THEN it SHALL connect to the known peers and join the cluster

#### Scenario: Gossip-based peer discovery

- GIVEN a node connected to at least one cluster member
- WHEN new nodes join the cluster
- THEN their endpoint information SHALL propagate via gossip
- AND all nodes SHALL eventually discover the new member
