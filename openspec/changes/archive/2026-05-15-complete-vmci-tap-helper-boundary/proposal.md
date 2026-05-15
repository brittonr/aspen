## Why

`dogfood-local-vmci` still fails after `setup-ci-network` because the unprivileged `aspen-node` process performs privileged TAP lifecycle operations (`ip tuntap`, bridge attach/up, delete) during VM boot and cleanup. Granting broad ambient `CAP_NET_ADMIN` to the node or running dogfood as root is too wide for a product acceptance rail.

## What Changes

- Implement the existing `NetworkMode::TapWithHelper` contract instead of falling back to direct TAP mode.
- Add a narrow `aspen-tap-helper` binary that only performs allowlisted CI TAP lifecycle operations for `ci-n*-vm*-tap` devices and bridge `aspen-ci-br0`.
- Make VM-CI dogfood prefer `tap-helper` when setup has installed the helper, while retaining explicit `tap`, `none`, and `isolated` modes.
- Update readiness/evidence so helper failures are deterministic and operator-actionable.

## Capabilities

### Modified Capabilities

- `dogfood-evidence`: VM-CI readiness and failure evidence distinguishes direct TAP privilege from helper-backed TAP lifecycle.

## Impact

- **Files**: `crates/aspen-ci-executor-vm`, `crates/aspen-dogfood`, `scripts/setup-ci-network.sh`, `flake.nix`, OpenSpec docs/tests.
- **APIs**: No public network API changes; fills existing `TapWithHelper` mode.
- **Security**: Narrows privilege to a helper command with name/bridge allowlists.
- **Testing**: Unit tests for allowlists/config/command selection; OpenSpec validation; focused VM executor/dogfood tests; live `dogfood-local-vmci` rerun when host helper is available.
