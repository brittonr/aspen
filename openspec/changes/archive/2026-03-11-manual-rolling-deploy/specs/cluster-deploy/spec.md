## ADDED Requirements

### Requirement: Rolling deployment orchestration

The system SHALL orchestrate rolling deployments across cluster nodes, upgrading one node at a time (or up to `max_concurrent`) while maintaining Raft quorum. Followers upgrade first, leader last.

#### Scenario: Deploy to a 3-node cluster

- **WHEN** a `ClusterDeploy` RPC is received with an artifact reference
- **THEN** the coordinator SHALL upgrade the two followers first, one at a time
- **AND** wait for health confirmation after each
- **AND** upgrade the leader last

#### Scenario: Deploy to a single-node cluster

- **WHEN** a `ClusterDeploy` RPC targets a single-voter cluster
- **THEN** the coordinator SHALL upgrade the single node directly
- **AND** accept brief cluster unavailability during restart

#### Scenario: Deploy with max_concurrent > 1

- **WHEN** deployment starts with `max_concurrent = 2` on a 5-node cluster
- **THEN** the coordinator SHALL upgrade up to 2 nodes simultaneously
- **AND** keep at least 3 nodes (quorum) operational at all times

### Requirement: Quorum safety invariant

The coordinator SHALL never reduce healthy voters below quorum. This SHALL be checked before each node upgrade.

#### Scenario: Quorum check before upgrade

- **WHEN** the coordinator is about to upgrade a node
- **THEN** it SHALL verify `(healthy_voters - nodes_upgrading - 1) >= quorum_size`
- **AND** if the check fails, wait for in-progress upgrades to complete

#### Scenario: Unrelated node failure during deployment

- **WHEN** an unrelated node failure reduces healthy voters during deployment
- **THEN** the coordinator SHALL pause until the failed node recovers or is removed

### Requirement: Deployment state machine

The system SHALL track deployment state in KV at `_sys:deploy:current` using CAS operations.

#### Scenario: State lifecycle

- **WHEN** a deployment is created
- **THEN** it SHALL transition: `pending` → `deploying` → `completed`
- **AND** each transition SHALL be a CAS write

#### Scenario: Leader failover during deployment

- **WHEN** the Raft leader changes while a deployment is in progress
- **THEN** the new leader SHALL read `_sys:deploy:current`
- **AND** resume from the last confirmed node upgrade
- **AND** NOT re-upgrade already-upgraded nodes

#### Scenario: Concurrent deployment rejection

- **WHEN** a `ClusterDeploy` arrives while another deployment has status `deploying`
- **THEN** the system SHALL return `DEPLOY_ALREADY_IN_PROGRESS`

### Requirement: Health-gated upgrade progression

The coordinator SHALL wait for each upgraded node to become healthy before proceeding.

#### Scenario: Node becomes healthy after upgrade

- **WHEN** a node restarts with the new binary
- **THEN** the coordinator SHALL verify: responds to `GetHealth`, appears in Raft membership, log gap < 100 entries
- **AND** only proceed to next node when all three pass

#### Scenario: Node fails health check

- **WHEN** a node does not pass health checks within `DEPLOY_HEALTH_TIMEOUT_SECS` (120)
- **THEN** the coordinator SHALL mark deployment `failed`
- **AND** record which nodes upgraded and which failed
- **AND** NOT upgrade additional nodes

### Requirement: Deployment rollback

The system SHALL support rolling back deployments.

#### Scenario: Rollback a failed deployment

- **WHEN** `ClusterRollback` is received and status is `failed`
- **THEN** the coordinator SHALL send `NodeRollback` RPCs to upgraded nodes
- **AND** transition status: `rolling_back` → `rolled_back`

#### Scenario: Rollback a completed deployment

- **WHEN** `ClusterRollback` is received and status is `completed`
- **THEN** the coordinator SHALL rollback all nodes using the same rolling strategy

#### Scenario: No deployment to rollback

- **WHEN** `ClusterRollback` arrives and no deployment exists
- **THEN** return `NO_DEPLOYMENT_FOUND`

### Requirement: Deployment status reporting

The system SHALL expose deployment progress via `ClusterDeployStatus` RPC.

#### Scenario: Query active deployment

- **WHEN** `ClusterDeployStatus` is received during a deployment
- **THEN** return: deploy_id, status, target artifact, per-node status list, start time, elapsed duration

#### Scenario: No active deployment

- **WHEN** `ClusterDeployStatus` is received with no active deployment
- **THEN** return the most recent deployment from history
- **AND** if no history, return `NO_DEPLOYMENT_FOUND`

### Requirement: Deployment history

The system SHALL maintain bounded history of past deployments.

#### Scenario: Record completed deployment

- **WHEN** a deployment completes (success, failure, or rollback)
- **THEN** write to `_sys:deploy:history:{timestamp}` with full record

#### Scenario: History pruning

- **WHEN** history exceeds `MAX_DEPLOY_HISTORY` (50) entries
- **THEN** prune oldest entries
