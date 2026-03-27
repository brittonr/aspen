## Why

The `dogfood-local.sh` script proves Aspen can build itself from a single cluster (Forge → CI → Nix build → deploy). Federation is wired and tested in NixOS VM isolation (`federation-ci-dogfood-test`), but nobody has run the federated dogfood loop on real processes — two independent clusters where one pushes code, the other syncs and builds it, then the build artifact gets deployed back. Closing this gap proves the full cross-cluster self-hosting pipeline works outside NixOS VM harnesses.

## What Changes

- New `scripts/dogfood-federation.sh` script that starts two independent local clusters (alice, bob), pushes Aspen source to alice's Forge, federates the repo, syncs it on bob, triggers CI on bob, and verifies the resulting binary.
- New `dogfood-serial-federation-vm` Nix package: a two-node bootable VM image for interactive `vm_boot` + `vm_serial` testing of the federated dogfood loop without rebuilding NixOS test framework.
- Fix any real-world bugs exposed by running federation sync + CI build on actual processes (not mocked, not VM-isolated).

## Capabilities

### New Capabilities

- `federated-dogfood-script`: Shell script orchestrating two-cluster federation dogfood — start, push, federate, sync, build, verify.
- `federated-dogfood-vm`: Bootable serial VM image with two aspen-node instances for interactive federation dogfood testing via pi's `vm_boot`/`vm_serial`.

### Modified Capabilities

## Impact

- `scripts/`: new `dogfood-federation.sh`
- `flake.nix`: new `dogfood-federation` app, new `dogfood-serial-federation-vm` package
- `nix/`: new VM image derivation
- May surface bugs in federation sync client, sync handler, or CI trigger-from-mirror paths when run on real iroh connections vs NixOS QEMU veth
