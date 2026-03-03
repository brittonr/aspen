# Net End-to-End VM Test

## What

Wire the aspen-net service mesh into a real Raft cluster and validate it end-to-end in a NixOS VM test. The test exercises the full path: publish services → resolve names → route TCP traffic through the SOCKS5 proxy over iroh QUIC tunnels.

## Why

The aspen-net crate has 45 unit/integration tests covering each component in isolation (registry, resolver, SOCKS5 handshake, auth, DNS records), but nothing validates them working together against a real Raft cluster. Three critical gaps:

1. **NetHandler is not registered** in the node's handler registry — `net publish` from CLI currently returns "no handler found". The handler exists but was never wired in.

2. **SOCKS5 tunnel is a placeholder** — after successful handshake and name resolution, the proxy sends a success reply but never creates an actual data tunnel through iroh QUIC. The comment says "requires a running iroh endpoint which is wired up in the daemon."

3. **Daemon doesn't connect to the cluster** — `NetDaemon::start()` creates a cancellation token and poll timer but never creates an iroh endpoint, never starts the SOCKS5 listener, and never connects to the Raft cluster for name resolution.

Fixing these gaps and proving them in a VM test is essential before the service mesh can be used for real inter-service routing.

## Scope

- Wire NetHandler into the node's handler registry (feature-gated on `net`)
- Wire the daemon to connect to the cluster via iroh client RPC
- Complete the SOCKS5 tunnel: resolve name → connect to endpoint via iroh → bidirectional copy
- Add `net` to node build features
- NixOS VM test: 3-node cluster, HTTP server on one node, `curl` through SOCKS5 proxy from another node
- Wire the test into flake.nix checks

## Out of scope

- DNS protocol server integration (requires aspen-dns feature, separate work)
- Port forwarding CLI (`aspen-net forward` — currently marked "not yet wired")
- `aspen-net down` / `aspen-net status` commands
- Token-based auth in the VM test (test uses cluster-internal paths)
