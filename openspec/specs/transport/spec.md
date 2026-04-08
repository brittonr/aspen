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
