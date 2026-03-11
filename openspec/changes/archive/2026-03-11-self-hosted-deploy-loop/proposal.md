## Why

Aspen can build itself (Forge → CI → Nix build) but the pipeline stops at "binary verified." The built binary sits in the blob store while the cluster keeps running the old one. Closing this loop — replacing the running nodes with the CI-built binary — makes Aspen truly self-hosting. Until nodes can upgrade themselves from their own build artifacts, the dogfood pipeline is a demo, not infrastructure.

## What Changes

- Add a **deploy stage** to the CI pipeline that takes build artifacts and rolls them across cluster nodes
- Implement **rolling node upgrade** orchestration: drain node → stop → replace binary → restart → health check → proceed to next
- Add a **`NodeUpgrade` RPC** that tells a node to replace its own binary and restart (via Nix profile switch or direct binary swap + `execve`)
- Add **`ClusterDeploy` coordination** that sequences upgrades across nodes while maintaining quorum
- Extend `dogfood-local.sh` with a `deploy` step that exercises the full loop
- Add a NixOS VM integration test that validates the complete Forge → CI → Nix build → deploy → verify cycle
- New CLI commands: `cluster deploy`, `cluster deploy-status`, `cluster rollback`

## Capabilities

### New Capabilities

- `node-upgrade`: Binary replacement and restart mechanism for a single node. Covers Nix profile switch, binary validation, graceful drain, process restart, and health verification.
- `cluster-deploy`: Cluster-wide rolling deployment orchestration. Covers quorum-safe sequencing, deployment state tracking, progress reporting, automatic rollback on health failure, and canary support.
- `deploy-stage`: CI pipeline deploy stage type. Covers deploy job configuration in `.aspen/ci.ncl`, artifact-to-deployment binding, and deployment trigger from CI completion.

### Modified Capabilities

- `ci`: Add `deploy` stage type to pipeline config. Jobs with `type = 'deploy` reference build artifacts and target the running cluster.
- `jobs`: Add `deploy` job type for the worker system to execute deployment tasks through the existing job framework.

## Impact

- **New crate**: `aspen-deploy` — deployment orchestration logic (rolling strategy, state machine, rollback)
- **Modified crates**:
  - `aspen-ci-core` — new `JobType::Deploy`, `DeployConfig` in pipeline types
  - `aspen-ci` — deploy executor that talks to cluster deploy coordinator
  - `aspen-client-api` — `ClusterDeploy*` RPC request/response variants
  - `aspen-cluster` — node-level upgrade execution (drain, binary swap, restart)
  - `aspen-cluster-handler` — handler for deploy RPCs
  - `aspen-cli` — `cluster deploy` / `cluster deploy-status` / `cluster rollback` commands
  - `aspen-constants` — deploy-related limits (max concurrent upgrades, health check timeout)
  - Root `Cargo.toml` — new `deploy` feature flag
- **Scripts**: `scripts/dogfood-local.sh` gains `deploy` and `full-loop` commands
- **NixOS tests**: New `ci-dogfood-deploy.nix` VM test
- **CI config**: `.aspen/ci.ncl` gains a stage 4 `deploy` block
- **Raft safety**: Deployment must never upgrade more than (N-1)/2 nodes simultaneously to preserve quorum
