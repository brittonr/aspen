# Federation Specification

## Purpose

Cross-cluster federation enabling independent Aspen clusters to share resources, replicate data, and coordinate operations. Any application built on Aspen (Forge, CI, jobs) can register resources for federation without coupling to a specific subsystem.

## Requirements

### Requirement: Federation Links

The system SHALL allow administrators to establish federation links between independent Aspen clusters. Links SHALL be mutual and authenticated via iroh endpoint IDs.

#### Scenario: Establish federation link

- GIVEN cluster A and cluster B are independent
- WHEN an administrator creates a federation link between them
- THEN both clusters SHALL recognize each other as federated peers
- AND communication SHALL flow over iroh QUIC

#### Scenario: Asymmetric federation

- GIVEN cluster A federates with cluster B
- WHEN cluster B does not reciprocate
- THEN cluster A MAY pull resources from B (if B allows)
- AND cluster B SHALL NOT pull resources from A

### Requirement: Resource Registration

The system SHALL allow any subsystem to register resources for federation. Resources SHALL be described by type, owner, and sync policy.

#### Scenario: Register a forge repository for federation

- GIVEN a repository `"infra-tools"` on cluster A
- WHEN an administrator marks it as federated
- THEN cluster B (linked) SHALL be able to discover and fetch it

#### Scenario: Register arbitrary resources

- GIVEN a subsystem `"monitoring"` with resource type `"dashboard"`
- WHEN the resource is registered for federation
- THEN federated peers SHALL discover it via the federation catalog

### Requirement: Federation Policy

The system SHALL enforce configurable policies governing what resources are shared, with whom, and in which direction (push, pull, or bidirectional).

#### Scenario: Read-only federation

- GIVEN cluster A shares repository `"public-lib"` as read-only
- WHEN cluster B attempts to push changes back
- THEN the push SHALL be rejected by cluster A's federation policy

#### Scenario: Selective sharing

- GIVEN cluster A has repositories `"public"` and `"private"`
- WHEN cluster A federates with cluster B sharing only `"public"`
- THEN cluster B SHALL see `"public"` but SHALL NOT see `"private"`

### Requirement: Federated Data Sync

The system SHALL synchronize federated resources across clusters using iroh-blobs for data transfer and Raft for metadata consistency within each cluster.

#### Scenario: Sync repository across clusters

- GIVEN repository `"shared-lib"` is federated between clusters A and B
- WHEN a new commit is pushed to cluster A
- THEN the commit objects SHALL replicate to cluster B within the sync interval
- AND cluster B's metadata SHALL be updated via its own Raft consensus
