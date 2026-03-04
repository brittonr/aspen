# MicroVM Net Mesh

## What

Run the aspen-net service mesh inside Cloud Hypervisor microVMs so guest workloads can discover and route to services registered in the Raft cluster. This mirrors how we already give microVMs a Raft-backed filesystem via AspenFs VirtioFS — now we give them a Raft-backed network layer too.

## Why

The microvm-raft-virtiofs test proves guest VMs can read/write files through the Raft cluster. But those guests have no way to discover or connect to other services in the mesh. A guest VM running a web app can't find its database, and a database VM can't advertise itself to clients.

Adding aspen-net to the microVM stack completes the picture: guests get both storage (VirtioFS → Raft) and networking (SOCKS5 → iroh QUIC → Raft service registry) from the cluster.

## Scope

- NixOS VM test: 3-node Raft cluster + aspen-net daemon on host + 2 CH microVMs
- Guest VM A runs an HTTP server and publishes it to the mesh
- Guest VM B uses the host SOCKS5 proxy to reach VM A's service by name
- Verifies the full path: guest B → TAP → host SOCKS5 → iroh tunnel → host TAP → guest A
- Wire test into flake.nix

## Out of Scope

- DNS inside guests (SOCKS5-hostname resolution is sufficient)
- mTLS / capability-based auth (permissive mode is fine for the test)
- Multi-host scenarios (single QEMU host with nested KVM is enough)
