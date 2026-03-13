## ADDED Requirements

### Requirement: Cluster formation through Home NAT

The system SHALL form a 3-node Raft cluster where all nodes are behind `RouterPreset::Home` NAT routers, relying on iroh's relay and hole-punching for connectivity.

#### Scenario: All nodes behind Home NAT elect a leader

- **WHEN** three nodes are spawned behind separate Home NAT routers and `init_cluster()` is called
- **THEN** a leader is elected within 60 seconds (allowing time for relay-based discovery and NAT traversal)

#### Scenario: KV write replicates across Home NAT

- **WHEN** a KV pair is written to the leader behind Home NAT
- **THEN** the value is readable from all three nodes within 10 seconds

### Requirement: Cluster formation through Corporate NAT

The system SHALL form a 3-node Raft cluster where nodes are behind `RouterPreset::Corporate` NAT (restricted outbound, TCP 80/443 + UDP 53 only).

#### Scenario: Nodes behind Corporate NAT with firewall

- **WHEN** three nodes are behind Corporate NAT routers
- **THEN** iroh falls back to relay-based communication and a Raft leader is elected within 60 seconds

### Requirement: Cluster formation through CGNAT

The system SHALL form a 3-node Raft cluster where nodes are behind `RouterPreset::IspCgnat` (carrier-grade NAT with multiple layers of address translation).

#### Scenario: Nodes behind CGNAT elect a leader

- **WHEN** three nodes are behind CGNAT routers
- **THEN** a leader is elected within 60 seconds via relay-assisted connectivity

#### Scenario: KV operations function through CGNAT

- **WHEN** a batch of 100 KV writes is sent to the leader behind CGNAT
- **THEN** all 100 values are replicated to all nodes within 30 seconds

### Requirement: Mixed NAT topology

The system SHALL form a Raft cluster where nodes are behind different NAT types (one public, one home, one corporate).

#### Scenario: Mixed NAT cluster formation

- **WHEN** node 1 is behind Public, node 2 behind Home NAT, node 3 behind Corporate NAT
- **THEN** all three nodes join the same Raft cluster and elect a leader within 60 seconds

#### Scenario: Leader failover across NAT boundaries

- **WHEN** the leader (behind Public) is shut down in a mixed NAT cluster
- **THEN** a new leader is elected from the remaining nodes (behind Home/Corporate NAT) within 30 seconds and KV writes continue to succeed

### Requirement: Public baseline topology

The system SHALL form a 3-node Raft cluster behind `RouterPreset::Public` routers as a control test confirming patchbay infrastructure works before testing NAT scenarios.

#### Scenario: Public baseline cluster formation

- **WHEN** three nodes are behind a public router with no NAT
- **THEN** a leader is elected within 30 seconds and a KV write/read round-trip succeeds
