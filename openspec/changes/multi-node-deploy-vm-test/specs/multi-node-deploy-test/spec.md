## ADDED Requirements

### Requirement: Three-node cluster formation with CI

The VM test SHALL boot 3 NixOS VMs, form a Raft cluster with all 3 as voters, and enable CI + workers on node1. Each node SHALL use a deterministic iroh secret key and shared cluster cookie. WASM plugins (KV, Forge) SHALL be installed on all nodes.

#### Scenario: Cluster forms successfully

- **WHEN** all 3 VMs boot and node1 runs `cluster init`, adds nodes 2 and 3 as learners, and promotes all to voters
- **THEN** `cluster status` reports 3 voters, node1 is leader, and `cluster health` succeeds on all nodes

### Requirement: CI pipeline with deploy stage completes

The test SHALL push a Nix flake to Forge with a CI config containing build and deploy stages. The pipeline SHALL auto-trigger on push, the build stage SHALL succeed, and the deploy stage SHALL execute via `DeploymentCoordinator`.

#### Scenario: Push triggers pipeline with deploy stage

- **WHEN** a flake with `.aspen/ci.ncl` (build + deploy stages) is pushed to a Forge repo with CI auto-trigger enabled
- **THEN** a pipeline is created and reaches `running` status within 30 seconds

#### Scenario: Build stage completes before deploy

- **WHEN** the pipeline runs
- **THEN** the build stage completes with status `succeeded` and produces a Nix store path artifact

#### Scenario: Deploy stage runs via DeploymentCoordinator

- **WHEN** the build stage succeeds
- **THEN** the deploy stage resolves the artifact from the build job, creates a deployment via `DeploymentCoordinator`, and the deploy stage reaches a terminal status

### Requirement: Deployment targets all cluster nodes

The `DeploymentCoordinator` SHALL create a deployment that targets all nodes in the cluster. The coordinator SHALL send `NodeUpgrade` RPCs to follower nodes before the leader (follower-first ordering).

#### Scenario: Deployment covers all 3 nodes

- **WHEN** the deploy stage initiates a rolling deployment
- **THEN** the deployment record in KV references all 3 cluster nodes

#### Scenario: Follower-first upgrade ordering

- **WHEN** the coordinator dispatches `NodeUpgrade` RPCs
- **THEN** nodes 2 and 3 (followers) receive upgrade RPCs before node 1 (leader)

### Requirement: Cluster remains healthy after deploy pipeline

The cluster SHALL remain operational after the deploy pipeline completes. All 3 nodes SHALL respond to health checks and serve KV requests.

#### Scenario: Health check passes on all nodes after deploy

- **WHEN** the deploy pipeline reaches a terminal status
- **THEN** `cluster health` succeeds when queried via any node's ticket

#### Scenario: KV data persists through deploy

- **WHEN** a KV key is written before the deploy pipeline starts
- **THEN** the same key can be read back with the correct value after the deploy pipeline completes

### Requirement: Test wired into flake checks

The VM test SHALL be accessible as a Nix flake check and excluded from quick-profile test runs.

#### Scenario: Test available as flake check

- **WHEN** a user runs `nix build .#checks.x86_64-linux.ci-dogfood-deploy-multinode-test`
- **THEN** the test builds and executes the full 3-node deploy scenario
