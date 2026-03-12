## MODIFIED Requirements

### Requirement: Deploy stage completes in multi-node cluster

The CI deploy stage SHALL complete successfully in multi-node clusters. The `DeploymentCoordinator` SHALL upgrade all nodes including the leader via leadership transfer, and the pipeline SHALL report the deploy stage as `succeeded`.

#### Scenario: Multi-node deploy stage succeeds end-to-end

- **WHEN** a CI pipeline with a deploy stage runs against a 3-node cluster
- **THEN** the build stage completes, the deploy stage upgrades all 3 nodes (followers via RPC, leader via transfer + resume), and the pipeline status is `success`

#### Scenario: Deploy stage reports node-level progress

- **WHEN** the deploy stage runs
- **THEN** coordinator logs show follower upgrades (nodes 2, 3) completing before leadership transfer, and the new leader completing the old leader's upgrade after transfer
