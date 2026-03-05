## Why

MicroVMs currently get either Raft-backed storage (VirtioFS via `microvm-raft-virtiofs`) or Raft-backed networking (SOCKS5 mesh via `microvm-net-mesh`), but never both. A guest VM running a web app can read its config from VirtioFS but can't discover its database through the service mesh, and vice versa. Combining both in a single microVM proves the full "serverless VM" story: guests get storage + networking from the cluster with zero guest-side configuration.

## What Changes

- **New NixOS VM test** (`nix/tests/microvm-virtiofs-net.nix`): Single test that boots a 3-node Raft cluster, starts both `aspen-cluster-virtiofs-server` and `aspen-net` daemon, launches a Cloud Hypervisor microVM with VirtioFS mount + TAP networking, and verifies the guest can simultaneously read files from VirtioFS and route HTTP traffic through the service mesh.
- **Guest VM with dual capabilities**: A CH microVM that mounts AspenFs via VirtioFS (for storage) and uses the host's SOCKS5 proxy (for service discovery/routing).
- **Two-guest scenario**: Guest A serves files from its VirtioFS mount over HTTP. Guest B reaches Guest A's HTTP server through the SOCKS5 service mesh. Content served originates from Raft KV store → VirtioFS → nginx → mesh → curl.
- **Flake integration**: Wire the test into `flake.nix` as `checks.x86_64-linux.microvm-virtiofs-net-test`.

## Capabilities

### New Capabilities

_No new library capabilities — this is a VM integration test proving existing capabilities compose._

### Modified Capabilities

_None — no spec-level behavior changes._

## Impact

- **New files**: `nix/tests/microvm-virtiofs-net.nix`
- **Modified files**: `flake.nix` (new check entry)
- **Dependencies**: Requires `microvm` flake input, `aspen-node-vm-test`, `aspen-cluster-virtiofs-server`, `aspen-net` daemon packages, `aspen-cli`
- **Hardware**: Requires nested KVM (x86_64-linux only)
- **Build time**: ~5-10 min (3-node cluster + VirtioFS daemon + 2 CH microVMs)
