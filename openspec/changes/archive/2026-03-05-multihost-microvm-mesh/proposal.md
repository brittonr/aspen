## Why

The `microvm-net-mesh` test proves service mesh routing works when all components (Raft cluster, aspen-net daemon, microVMs) run on a single QEMU host. But real distributed systems span multiple machines. The iroh QUIC tunnel should transparently route traffic between microVMs on different hosts — that's the whole point of a service mesh. We need a multi-host VM test to prove cross-host routing works end-to-end.

## What Changes

- **New NixOS VM test** (`nix/tests/multihost-microvm-mesh.nix`): Multi-QEMU-node test where Host A and Host B each run aspen-net daemons connected to a shared Raft cluster, each with their own CH microVM. Guest A on Host A publishes an HTTP service. Guest B on Host B reaches it through SOCKS5 → iroh QUIC tunnel → across QEMU network → Host A TunnelAcceptor → Guest A.
- **Multi-host Raft cluster**: 3 aspen-node processes spread across 2 QEMU nodes (2 on host A, 1 on host B) proving cross-host consensus.
- **Cross-host tunnel verification**: The iroh tunnel between Host B's SOCKS5 proxy and Host A's TunnelAcceptor traverses a real network link (QEMU virtual bridge), not loopback.
- **Flake integration**: Wire into `flake.nix` as `checks.x86_64-linux.multihost-microvm-mesh-test`.

## Capabilities

### New Capabilities

_No new library capabilities — this is a VM integration test proving existing cross-host behavior._

### Modified Capabilities

_None — no spec-level behavior changes._

## Impact

- **New files**: `nix/tests/multihost-microvm-mesh.nix`
- **Modified files**: `flake.nix` (new check entry)
- **Dependencies**: Requires `microvm` flake input, all aspen packages, multi-node NixOS test driver
- **Hardware**: Requires nested KVM (x86_64-linux only)
- **Networking**: QEMU virtual bridge between 2 host VMs, each with their own TAP interfaces for CH microVMs
- **Complexity**: Higher than single-host tests — needs careful IP planning (host-to-host network + per-host guest TAP subnets)
- **Build time**: ~10-15 min (multi-node cluster + 2 hosts + 2 CH microVMs)
