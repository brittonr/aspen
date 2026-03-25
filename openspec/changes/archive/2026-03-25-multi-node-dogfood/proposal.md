## Why

The dogfood pipeline (`dogfood-local.sh`) defaults to `ASPEN_NODE_COUNT=1`. The multi-node path (add-learner, change-membership, rolling deploy) exists in the script and in the `DeploymentCoordinator`, but has never been exercised end-to-end. A single-node cluster skips Raft replication, leader-last upgrade ordering, quorum safety enforcement, and leadership transfer during deploy. We need a 3-node NixOS VM integration test that proves the full pipeline works under real consensus conditions: replicated state, follower-first upgrades, and health-gated progression.

## What Changes

- **3-node NixOS VM test** (`nix/tests/multi-node-dogfood.nix`): A new NixOS VM integration test that boots a 3-node Raft cluster across separate VMs, pushes source to Forge, runs a CI pipeline, and deploys the built artifact via rolling upgrade. Validates Raft log replication, follower-first ordering, and cluster health after deploy.
- **dogfood-local.sh multi-node fixes**: Fix any issues found when running `ASPEN_NODE_COUNT=3 nix run .#dogfood-local -- full-loop`. The script already has multi-node paths but they haven't been tested since initial authoring.
- **Deploy coordinator hardening**: Address any issues discovered during multi-node deploy — leader transfer timing, health check reliability, or state persistence across failover.

## Capabilities

### New Capabilities

- `multi-node-deploy-test`: NixOS VM integration test proving 3-node rolling deploy works end-to-end (Forge push → CI build → rolling deploy across 3 nodes → health verification)

### Modified Capabilities

## Impact

- `nix/tests/`: New VM test file, new flake check entry
- `scripts/dogfood-local.sh`: Bug fixes for multi-node paths if any are found
- `crates/aspen-deploy/`: Potential fixes to coordinator if multi-node deploy reveals issues
- `flake.nix`: New check target for the VM test
