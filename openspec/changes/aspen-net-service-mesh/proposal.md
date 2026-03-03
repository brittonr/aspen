## Why

Aspen has all the low-level networking primitives — iroh provides encrypted P2P QUIC with NAT traversal, and `iroh-proxy-utils` already bridges TCP traffic over those connections with connection pooling, auth, and metrics. But there's no way for a user to say "connect me to the database" without knowing endpoint IDs and constructing proxy configurations manually.

Tailscale proved that the killer feature of a modern VPN isn't the encryption — it's the **service discovery and naming**. You name your machines, tag them, set ACLs, and `ssh myserver` just works. Aspen needs the same UX layer, but built natively on iroh rather than emulating IP networking.

A traditional VPN approach (TUN device, IP allocation, IP routing) would fight against iroh's model — it would double-encrypt traffic (IP-in-QUIC), require root/CAP_NET_ADMIN, create MTU headaches, and duplicate work iroh already handles. Instead, we build a **service mesh** that exposes iroh's connection model to existing TCP applications through proxying, naming, and access control.

## What Changes

- New `aspen-net` crate providing a service mesh control plane over iroh-proxy-utils' existing TCP↔QUIC data plane
- Service registry in Raft KV: services registered by name, discovered by name, resolved to iroh endpoint + port
- SOCKS5 proxy server that resolves `*.aspen` service names and tunnels TCP connections through iroh QUIC via `DownstreamProxy::create_tunnel()`
- Port forwarding command that wraps `DownstreamProxy` with `ProxyMode::Tcp` for direct local-port-to-remote-service mapping
- Authorization via existing UCAN capability tokens (`aspen-auth`): new `NetConnect` and `NetPublish` capability variants enable decentralized, offline-verifiable access control with delegation chains — no central ACL store needed
- MagicDNS stub resolver for `*.aspen` domains mapping to loopback addresses
- Daemon mode (`aspen net up/down`) orchestrating SOCKS5 + DNS + service publishing
- CLI subcommands under `aspen net` for publish, forward, proxy, status, and peers

## Capabilities

### New Capabilities

- `net-registry`: Service registration and discovery via Raft KV. Services are published by name with endpoint ID, port, protocol, tags, and optional hostname. Lookup resolves names to connection targets.
- `net-socks5`: Local SOCKS5 proxy server (RFC 1928) that resolves `*.aspen` service names via the registry and creates CONNECT tunnels through iroh-proxy-utils' `DownstreamProxy`.
- `net-forward`: TCP port forwarding that maps a local port to a named remote service, wrapping `DownstreamProxy` with `ProxyMode::Tcp`.
- `net-auth`: Authorization via existing UCAN capability tokens (`aspen-auth`). New `NetConnect { service_prefix }` and `NetPublish { service_prefix }` capability variants. Daemon verifies its token locally at connect/publish time — no Raft read needed. Supports delegation chains (admin → team → CI runner with attenuated access).
- `net-dns`: MagicDNS stub resolver for `*.aspen` domains. Maps service names to loopback addresses, enabling `curl http://mydb.aspen:8080` without proxy configuration.
- `net-daemon`: Long-running daemon orchestrating SOCKS5 proxy, DNS resolver, and service auto-publishing. Started with `aspen net up`, stopped with `aspen net down`.

### Modified Capabilities

- `cli`: New `aspen net` subcommand group with publish, unpublish, forward, proxy, status, peers, dns, up, down.
- `client-api`: New RPC variants for service mesh operations (NetPublish, NetUnpublish, NetLookup, NetList).
- `auth`: New `Capability` variants (`NetConnect`, `NetPublish`, `NetAdmin`) and corresponding `Operation` variants in `aspen-auth`.
- `rpc-handlers`: Registration of net service handler in the handler registry.
- `constants`: Network constants for service mesh (max services, SOCKS5 limits, DNS TTLs).

## Impact

- **Dependencies**: `fast-socks5` or hand-rolled SOCKS5 (~200 lines), `hickory-dns` for DNS stub resolver. No new heavy dependencies — the data plane is entirely `iroh-proxy-utils` (already vendored).
- **New crate**: `aspen-net` (~1,200 lines) — thin control plane over existing proxy infrastructure. Simpler than originally planned since auth delegates to `aspen-auth` capability tokens.
- **No kernel interface**: No TUN device, no root required, no IP allocation. Purely userspace.
- **No new ALPN**: Uses existing `iroh-http-proxy/1` ALPN from iroh-proxy-utils for all tunneling.
- **No central ACL store**: Authorization is token-based via `aspen-auth`. No `/_sys/net/acl/` keys needed. Token verification is local and offline.
- **Raft KV schema**: New `/_sys/net/` key prefix for service registry and DNS overrides only.
- **Auth changes**: New `Capability` and `Operation` variants in `aspen-auth` (additive, no breaking changes).
- **Feature gated**: Behind `net` feature flag so it's opt-in.
