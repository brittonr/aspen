## ADDED Requirements

### Requirement: Rolling deployment orchestration

The system SHALL orchestrate rolling deployments across cluster nodes, upgrading one node at a time (or up to `max_concurrent`) while maintaining Raft quorum.

#### Scenario: Deploy to a 3-node cluster

- **WHEN** a `ClusterDeploy` RPC is received with an artifact reference and `strategy = rolling`
- **THEN** the coordinator SHALL upgrade nodes one at a time in order: followers first, leader last
- **AND** after each node upgrade, wait for health confirmation before proceeding to the next
- **AND** never have more than `(N-1)/2` nodes upgrading simultaneously where N is voter count

#### Scenario: Deploy to a single-node cluster

- **WHEN** a `ClusterDeploy` RPC is received for a cluster with one voter
- **THEN** the coordinator SHALL upgrade the single node directly
- **AND** accept that the cluster is briefly unavailable during restart

#### Scenario: Deploy with max_concurrent > 1

- **WHEN** a deployment is started with `max_concurrent = 2` on a 5-node cluster
- **THEN** the coordinator SHALL upgrade up to 2 nodes simultaneously
- **AND** enforce that at least 3 nodes (quorum) remain operational at all times

### Requirement: Quorum safety invariant

The deployment coordinator SHALL never reduce the number of healthy voters below quorum. This invariant SHALL be checked before initiating each node upgrade.

#### Scenario: Quorum check before upgrade

- **WHEN** the coordinator is about to upgrade node N
- **THEN** it SHALL verify that `(healthy_voters - nodes_currently_upgrading - 1) >= quorum_size`
- **AND** if the check fails, the coordinator SHALL wait for in-progress upgrades to complete before proceeding

#### Scenario: Node failure during deployment

- **WHEN** an unrelated node failure reduces healthy voters during a deployment
- **THEN** the coordinator SHALL pause the deployment until the failed node recovers or is removed from membership
- **AND** log a warning about the reduced cluster capacity

### Requirement: Deployment state machine

The system SHALL track deployment state in KV storage at `_sys:deploy:current` using CAS operations for consistency across leader failover.

#### Scenario: Deployment state lifecycle

- **WHEN** a deployment is created
- **THEN** it SHALL transition through states: `pending` → `deploying` → `completed`
- **AND** each state transition SHALL be a CAS write to `_sys:deploy:current`

#### Scenario: Leader failover during deployment

- **WHEN** the Raft leader changes while a deployment is in progress
- **THEN** the new leader SHALL read `_sys:deploy:current` to recover deployment state
- **AND** resume the deployment from the last confirmed node upgrade
- **AND** NOT re-upgrade nodes that have already been upgraded

#### Scenario: Concurrent deployment rejection

- **WHEN** a `ClusterDeploy` RPC is received while another deployment is in progress (status = `deploying`)
- **THEN** the system SHALL reject the new deployment with `DEPLOY_ALREADY_IN_PROGRESS` error

### Requirement: Health-gated upgrade progression

The coordinator SHALL wait for each upgraded node to become healthy before proceeding to the next node.

#### Scenario: Node becomes healthy after upgrade

- **WHEN** a node restarts with the new binary
- **THEN** the coordinator SHALL poll `GetHealth` RPC on the upgraded node
- **AND** verify the node appears in Raft membership
- **AND** verify the node's Raft log gap is less than 100 entries
- **AND** only mark the node as `upgraded` when all three checks pass

#### Scenario: Node fails health check after upgrade

- **WHEN** a node does not pass health checks within `DEPLOY_HEALTH_TIMEOUT_SECS` (default 120)
- **THEN** the coordinator SHALL mark the deployment as `failed`
- **AND** record which nodes were successfully upgraded and which failed
- **AND** NOT proceed to upgrade additional nodes

### Requirement: Deployment rollback

The system SHALL support rolling back a failed or completed deployment to the previous binary version.

#### Scenario: Rollback a failed deployment

- **WHEN** a `ClusterRollback` RPC is received and the current deployment status is `failed`
- **THEN** the coordinator SHALL send `NodeRollback` RPCs to all nodes that were upgraded in this deployment
- **AND** transition the deployment status to `rolling_back` → `rolled_back`

#### Scenario: Rollback a completed deployment

- **WHEN** a `ClusterRollback` RPC is received and the current deployment status is `completed`
- **THEN** the coordinator SHALL send `NodeRollback` RPCs to all cluster nodes
- **AND** follow the same rolling strategy (one at a time, health-gated)

#### Scenario: No deployment to rollback

- **WHEN** a `ClusterRollback` RPC is received and no deployment exists
- **THEN** the system SHALL return `NO_DEPLOYMENT_FOUND` error

### Requirement: Deployment progress reporting

The system SHALL expose deployment progress via RPC so operators and CI jobs can monitor status.

#### Scenario: Query deployment status

- **WHEN** a `ClusterDeployStatus` RPC is received
- **THEN** the system SHALL return the current deployment state including: deploy_id, status, target artifact, list of nodes with per-node status (pending/upgrading/upgraded/failed), start time, and elapsed duration

#### Scenario: No active deployment

- **WHEN** a `ClusterDeployStatus` RPC is received and no deployment exists
- **THEN** the system SHALL return the most recent completed or failed deployment from history
- **AND** if no history exists, return `NO_DEPLOYMENT_FOUND`

### Requirement: Deployment history

The system SHALL maintain a bounded history of past deployments in KV storage.

#### Scenario: Record completed deployment

- **WHEN** a deployment completes (success, failure, or rollback)
- **THEN** the system SHALL write the deployment record to `_sys:deploy:history:{timestamp}`
- **AND** the record SHALL include: deploy_id, artifact reference, final status, per-node outcomes, start/end timestamps

#### Scenario: History retention limit

- **WHEN** deployment history exceeds `MAX_DEPLOY_HISTORY` (default 50) entries
- **THEN** the oldest entries SHALL be pruned
