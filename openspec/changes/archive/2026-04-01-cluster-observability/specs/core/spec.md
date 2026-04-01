## MODIFIED Requirements

### Requirement: Cluster Controller

The system SHALL expose a `ClusterController` trait for managing Raft cluster membership: initializing a cluster, adding learners, changing membership, and querying metrics.

#### Scenario: Initialize single-node cluster

- GIVEN a fresh Aspen node with no cluster state
- WHEN the node initializes a cluster with itself as the sole member
- THEN the node SHALL become leader
- AND the cluster SHALL accept read/write operations

#### Scenario: Add learner node

- GIVEN a running single-node cluster
- WHEN a second node is added as a learner
- THEN the learner SHALL replicate the leader's log
- AND the learner SHALL NOT participate in elections until promoted to voter

#### Scenario: Change membership

- GIVEN a cluster with nodes {1, 2, 3}
- WHEN membership is changed to {1, 2, 3, 4, 5}
- THEN the new configuration SHALL take effect after Raft consensus
- AND the cluster SHALL continue operating during the transition

#### Scenario: GetMetrics returns registry-generated Prometheus text

- **WHEN** a client sends `GetMetrics`
- **THEN** the response `prometheus_text` field SHALL contain output generated from the `metrics` crate registry
- **AND** the output SHALL include all registered metrics (RPC latency, connection pool gauges, write batcher counters, snapshot transfer histograms) without hardcoded format strings
