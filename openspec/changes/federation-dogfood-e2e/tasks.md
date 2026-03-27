## 1. Federation Dogfood Script

- [x] 1.1 Create `scripts/dogfood-federation.sh` with two-cluster lifecycle (start alice + bob, stop, status)
- [x] 1.2 Implement `push` subcommand: create Forge repo on alice, push Aspen source via git-remote-aspen
- [x] 1.3 Implement `federate` subcommand: mark alice's repo as federated (public mode)
- [x] 1.4 Implement `sync` subcommand: extract alice's iroh NodeId + address, run `federation trust` + `federation sync` on bob
- [x] 1.5 Implement `build` subcommand: create bob-side Forge repo, push mirrored content, wait for CI pipeline
- [x] 1.6 Implement `verify` subcommand: extract nix output path from CI job, run the built binary
- [x] 1.7 Implement `full` subcommand: chain start → push → federate → sync → build → verify
- [x] 1.8 Wire `dogfood-federation` app in `flake.nix` with correct binary deps

## 2. Serial VM Image

- [x] 2.1 Create `nix/vm/dogfood-serial-federation.nix` NixOS config with aspen-node-alice and aspen-node-bob systemd services
- [x] 2.2 Add helper scripts at `/etc/dogfood-federation/` (init-clusters.sh, push-source.sh, federate.sh, sync.sh, build.sh, verify.sh, full-test.sh)
- [x] 2.3 Wire `dogfood-serial-federation-vm` package in `flake.nix` producing qcow2

## 3. Test and Fix

- [x] 3.1 Run `nix run .#dogfood-federation` on host, fix any federation sync or CI trigger bugs exposed
- [ ] 3.2 Build and boot VM via `vm_boot`, run full pipeline via `vm_serial`, fix any VM-specific issues
- [x] 3.3 Verify existing federation NixOS tests still pass (`federation-test`, `federation-git-clone-test`)
