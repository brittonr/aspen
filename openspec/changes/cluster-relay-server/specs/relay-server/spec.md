## ADDED Requirements

### Requirement: Embedded Relay Server

Each Raft node in the cluster SHALL run an embedded iroh relay server when the `relay-server` feature is enabled. The relay server SHALL accept connections from authorized endpoints and forward encrypted traffic between peers.

#### Scenario: Relay server starts with node

- **WHEN** an aspen node starts with the `relay-server` feature enabled
- **THEN** the node SHALL spawn an iroh relay HTTP server on the configured relay bind address
- **AND** the relay server SHALL be listening before the iroh endpoint's relay mode is configured

#### Scenario: Relay server disabled by feature flag

- **WHEN** an aspen node starts without the `relay-server` feature
- **THEN** no relay server SHALL be spawned
- **AND** the node SHALL use its configured relay mode (default, custom, or disabled) as before

#### Scenario: Relay server shuts down with node

- **WHEN** a node receives a shutdown signal
- **THEN** the relay server SHALL be shut down gracefully before the iroh endpoint closes
- **AND** existing relay connections SHALL be drained with a bounded timeout

### Requirement: Cluster-Aware Access Control

The relay server SHALL restrict access to endpoints that are members of the Raft cluster or known peers. Unauthorized endpoints SHALL be denied relay access.

#### Scenario: Cluster member connects to relay

- **GIVEN** a node whose endpoint ID is present in the Raft membership
- **WHEN** the node connects to the relay server
- **THEN** the connection SHALL be accepted

#### Scenario: Known gossip peer connects to relay

- **GIVEN** a peer whose endpoint ID is in the node's known peer set
- **WHEN** the peer connects to the relay server
- **THEN** the connection SHALL be accepted

#### Scenario: Unknown endpoint denied

- **GIVEN** an endpoint ID that is not in the Raft membership or known peer set
- **WHEN** the endpoint attempts to connect to the relay server
- **THEN** the connection SHALL be denied

### Requirement: Dynamic Relay Map from Membership

Nodes SHALL construct their iroh `RelayMap` from the current Raft cluster membership. The relay map SHALL update when membership changes (nodes join or leave).

#### Scenario: Initial relay map construction

- **GIVEN** a cluster with 3 nodes, each running a relay server
- **WHEN** a node constructs its relay map
- **THEN** the map SHALL contain relay URLs for all 3 nodes

#### Scenario: Node joins cluster

- **GIVEN** a running cluster with an established relay map
- **WHEN** a new node joins the Raft cluster
- **THEN** all existing nodes SHALL add the new node's relay URL to their relay map

#### Scenario: Node leaves cluster

- **GIVEN** a running cluster with an established relay map
- **WHEN** a node is removed from the Raft cluster
- **THEN** all remaining nodes SHALL remove the departed node's relay URL from their relay map

### Requirement: Relay URL in Node Metadata

Each node's relay server URL SHALL be stored in its Raft node metadata alongside the existing iroh endpoint address. This allows any node to discover all relay URLs from the membership state.

#### Scenario: Node metadata includes relay URL

- **GIVEN** a node running a relay server on `http://192.168.1.10:3340`
- **WHEN** the node's metadata is inspected
- **THEN** the metadata SHALL include the relay URL

#### Scenario: Node without relay server

- **GIVEN** a node not running a relay server (feature disabled)
- **WHEN** the node's metadata is inspected
- **THEN** the relay URL field SHALL be empty or absent

### Requirement: Relay Server Configuration

The relay server SHALL be configurable via the node's configuration file with sensible defaults. Configuration SHALL include bind address, port, rate limits, and key cache capacity.

#### Scenario: Default configuration

- **WHEN** no relay server configuration is specified
- **THEN** the relay server SHALL bind to `0.0.0.0:3340`
- **AND** rate limits SHALL be disabled (unlimited)
- **AND** key cache capacity SHALL be 1024

#### Scenario: Custom port configuration

- **GIVEN** a configuration with `relay_server.bind_port = 4000`
- **WHEN** the relay server starts
- **THEN** it SHALL bind to port 4000

#### Scenario: Rate limit configuration

- **GIVEN** a configuration with client rate limits set
- **WHEN** a client sends data through the relay
- **THEN** the data rate SHALL be limited according to the configured limits

### Requirement: Self-Hosted Relay Mode

When the relay server feature is enabled, nodes SHALL automatically configure their iroh endpoint to use the cluster's own relay servers instead of external infrastructure. This SHALL override the default relay mode.

#### Scenario: Auto-configure custom relay mode

- **GIVEN** a cluster with relay servers running on all nodes
- **WHEN** a node configures its iroh endpoint
- **THEN** it SHALL use `RelayMode::Custom` with a `RelayMap` containing all cluster relay URLs

#### Scenario: Fallback during bootstrap

- **GIVEN** the first node in a new cluster with no other relay servers available
- **WHEN** the node starts
- **THEN** it SHALL use its own relay URL as the sole entry in the relay map
- **AND** the relay map SHALL expand as other nodes join

#### Scenario: Explicit relay mode override

- **GIVEN** a node with relay server enabled but `relay_mode` explicitly set to `disabled`
- **WHEN** the node starts
- **THEN** the explicit relay mode SHALL take precedence
- **AND** the node SHALL still run the relay server for other nodes to use
