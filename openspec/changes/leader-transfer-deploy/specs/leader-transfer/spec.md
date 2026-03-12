## ADDED Requirements

### Requirement: ClusterController exposes leadership transfer

The `ClusterController` trait SHALL provide a `transfer_leader(target: u64)` method that triggers Raft leadership transfer to the specified node. The method SHALL return `Ok(())` once the transfer is initiated. The `RaftNode` implementation SHALL delegate to openraft's `trigger().transfer_leader()`.

#### Scenario: Transfer to a healthy voter

- **WHEN** `transfer_leader(node_id)` is called with a node that is a voter in the current membership
- **THEN** the method returns `Ok(())` and the target node becomes the new Raft leader

#### Scenario: Transfer to non-existent node

- **WHEN** `transfer_leader(node_id)` is called with a node ID not in the cluster membership
- **THEN** the method returns an error

### Requirement: Deploy coordinator transfers leadership before self-upgrade

The `DeploymentCoordinator` SHALL transfer Raft leadership to an already-upgraded follower before attempting to upgrade the leader node. After transfer, the coordinator SHALL return early without attempting the leader's `upgrade_single_node()`.

#### Scenario: Leadership transferred to upgraded follower

- **WHEN** all followers are upgraded and healthy, and the leader is next
- **THEN** the coordinator transfers leadership to the first healthy follower, persists the deployment record with the leader's status as `Pending`, and returns

#### Scenario: No healthy follower available for transfer

- **WHEN** the coordinator needs to upgrade the leader but no follower has status `Healthy`
- **THEN** the deployment fails with an error indicating no transfer target is available

#### Scenario: Single-node cluster

- **WHEN** the deployment has only one node (the leader) and no followers exist
- **THEN** the coordinator returns an error indicating single-node self-upgrade is not supported via transfer

### Requirement: New leader resumes deployment after transfer

After leadership transfer, the new leader's `leader_resume.rs` watcher SHALL detect the transition, find the in-progress deployment in KV, and resume it. The resume logic SHALL upgrade the old leader (now a follower) via normal iroh RPC and finalize the deployment.

#### Scenario: Resume upgrades old leader as follower

- **WHEN** a node becomes leader and finds a `Deploying` record where one node has `Pending` status
- **THEN** the new leader sends `NodeUpgrade` RPC to the pending node, polls its health, marks it `Healthy`, and finalizes the deployment as `Completed`

#### Scenario: Old leader unreachable after transfer

- **WHEN** the new leader attempts to upgrade the old leader (now follower) and the node is unreachable
- **THEN** the deployment is marked `Failed` with the node's failure reason
