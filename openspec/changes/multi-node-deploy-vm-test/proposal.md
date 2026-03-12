## Why

The deploy stage was just wired into the CI pipeline, but the only VM test (`ci-dogfood-deploy.nix`) runs a 1-node cluster where `DeploymentCoordinator` fails with "Connecting to ourself is not supported" — iroh can't send RPCs to the local node. The dogfood-local script works around this with a script-level binary swap, but the actual coordinator code path (follower-first ordering, quorum safety, drain → upgrade → health per node) has never been exercised in a VM test. A 3-node test would prove the real deploy path works.

## What Changes

- New NixOS VM integration test with a 3-node cluster that exercises the full `DeploymentCoordinator` rolling upgrade path: Forge push → CI auto-trigger → nix build → deploy stage → coordinator upgrades followers then leader → cluster healthy
- The deploy stage succeeds (not fails-as-expected like the 1-node test)
- Validates quorum is maintained throughout the rolling upgrade
- Verifies KV data survives the upgrade (write before deploy, read after)

## Capabilities

### New Capabilities

- `multi-node-deploy-test`: NixOS VM integration test proving the DeploymentCoordinator's rolling upgrade path works across a 3-node cluster, with follower-first ordering, quorum safety, and data persistence through upgrades

### Modified Capabilities

## Impact

- New file: `nix/tests/ci-dogfood-deploy-multinode.nix`
- `flake.nix`: Wire the new test into `checks.x86_64-linux`
- Builds on existing patterns from `forge-cluster.nix` (multi-node setup) and `ci-dogfood-deploy.nix` (deploy pipeline)
- RAM-heavy test (~3 VMs × 2-4GB each) — should be in the `ci` nextest profile equivalent for VM tests, not the quick path
