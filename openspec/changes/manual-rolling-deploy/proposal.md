## Why

Aspen clusters have no way to upgrade their own binaries. Operators must SSH into each node, replace the binary, and restart — manually coordinating to avoid losing quorum. This is the first phase of closing the self-hosted loop: build the machinery for quorum-safe rolling upgrades, driven from the CLI.

Phase 2 (`ci-deploy-loop`) will wire this into the CI pipeline for fully automated Forge → CI → Build → Deploy.

## What Changes

- Implement **node-level binary replacement**: graceful drain → Nix profile switch (or blob-based binary swap) → restart → health report
- Implement **cluster-level rolling deploy coordinator**: leader-coordinated, quorum-safe, health-gated progression through nodes one at a time
- Add **deployment state machine in KV** with CAS for leader failover safety
- Add **rollback support**: revert upgraded nodes to previous binary
- New RPC messages: `ClusterDeploy`, `ClusterDeployStatus`, `ClusterRollback`, `NodeUpgrade`, `NodeRollback`
- New CLI commands: `cluster deploy`, `cluster deploy-status`, `cluster rollback`
- New crate: `aspen-deploy` for deployment orchestration

## Capabilities

### New Capabilities

- `node-upgrade`: Binary replacement and restart mechanism for a single node. Covers Nix profile switch, blob-based fallback, graceful drain, process restart, and health verification.
- `cluster-deploy`: Cluster-wide rolling deployment orchestration. Covers quorum-safe sequencing, deployment state tracking, progress reporting, automatic halt on health failure, and rollback.

### Modified Capabilities

- `jobs`: Deploy job type routes to the deployment coordinator on the leader, not to regular worker pools.

## Impact

- **New crate**: `aspen-deploy` — deployment state machine, rolling strategy, quorum safety, rollback
- **Modified crates**:
  - `aspen-client-api` — `ClusterDeploy*`, `NodeUpgrade*`, `NodeRollback*` RPC variants
  - `aspen-cluster` — `src/upgrade/` module for node-level drain, binary swap, restart
  - `aspen-cluster-handler` — handlers for deploy and upgrade RPCs
  - `aspen-rpc-handlers` — dispatch for new handlers under `deploy` feature flag
  - `aspen-cli` — `cluster deploy` / `cluster deploy-status` / `cluster rollback`
  - `aspen-constants` — `DRAIN_TIMEOUT_SECS`, `DEPLOY_HEALTH_TIMEOUT_SECS`, `MAX_DEPLOY_HISTORY`
  - Root `Cargo.toml` — new `deploy` feature flag
- **No CI config changes** — that's phase 2
- **No dogfood script changes** — that's phase 2
