# MicroVM Net Mesh — Design

## Architecture

```
┌─────────────────────── QEMU Host ───────────────────────┐
│                                                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │ aspen-1  │ │ aspen-2  │ │ aspen-3  │  Raft cluster  │
│  │ :7001    │ │ :7002    │ │ :7003    │  (inmemory)    │
│  └──────────┘ └──────────┘ └──────────┘                │
│        ↕ iroh QUIC                                      │
│  ┌──────────────────┐  ┌──────────────────┐            │
│  │ aspen-net daemon │  │ aspenfs-virtiofs │            │
│  │ SOCKS5 :1080     │  │ /tmp/aspenfs.sock│            │
│  │ TunnelAcceptor   │  └──────────────────┘            │
│  └────────┬─────────┘                                   │
│           │                                              │
│   ┌───────┼────────────────────────┐                    │
│   │       │ TAP networking         │                    │
│   │  ┌────┴─────┐   ┌───────────┐ │                    │
│   │  │ vm-svc   │   │ vm-client │ │                    │
│   │  │ 10.10.1.2│   │ 10.10.2.2 │ │                    │
│   │  └──────────┘   └───────────┘ │                    │
│   │  CH guest A      CH guest B   │                    │
│   └────────────────────────────────┘                    │
└──────────────────────────────────────────────────────────┘
```

## Data Flow

### Service Registration (Guest A → Raft)

1. Host publishes guest A's HTTP service via `aspen-cli net publish`
2. CLI sends NetPublish RPC → Raft leader stores service entry in KV

### Service Discovery + Routing (Guest B → Guest A)

1. Guest B runs `curl --socks5-hostname <host>:1080 http://my-svc.aspen:8080/`
2. Traffic exits guest B via TAP to host
3. Host SOCKS5 proxy receives connection, resolves `my-svc` → endpoint_id + port
4. SOCKS5 opens iroh QUIC tunnel to the node's TunnelAcceptor (NET_TUNNEL_ALPN)
5. TunnelAcceptor connects to `127.0.0.1:8080` (the forwarded port from guest A's TAP)
6. Bidirectional copy: guest B TCP ↔ host SOCKS5 ↔ iroh QUIC ↔ TunnelAcceptor ↔ guest A TCP

### Key Insight: Port Forwarding

Guest A's HTTP server listens on `10.10.1.2:8080` inside the VM. From the host, this is reachable at `10.10.1.2:8080` via the TAP interface. The service entry's `endpoint_id` points to the aspen node (which has the TunnelAcceptor), and the TunnelAcceptor connects to `127.0.0.1:{port}`.

**Problem**: TunnelAcceptor connects to `127.0.0.1:{port}` but guest A's service is at `10.10.1.2:{port}` from the host's perspective.

**Solution**: Use iptables DNAT on the host to forward `127.0.0.1:{guest_port}` → `10.10.1.2:{guest_port}`, OR set up a socat/port-forward from localhost to the guest IP. Simpler: run a lightweight TCP forwarder on the host (`socat TCP-LISTEN:8080,fork TCP:10.10.1.2:8080`).

## Networking Layout

- **TAP vm-svc**: Guest A network, host side `10.10.1.1/24`, guest `10.10.1.2/24`
- **TAP vm-client**: Guest B network, host side `10.10.2.1/24`, guest `10.10.2.2/24`
- **SOCKS5**: Host `127.0.0.1:1080`, guest B uses `10.10.2.1:1080` (host's TAP IP)

## Guest VM Configuration

Both guests are minimal NixOS Cloud Hypervisor microVMs:

- 512MB RAM, 1 vCPU
- TAP networking with static IPs
- Guest A: python3 HTTP server on port 8080
- Guest B: curl with SOCKS5 support

## Test Phases

1. **Bootstrap**: 3-node Raft cluster (same pattern as microvm-raft-virtiofs)
2. **Net daemon**: Start aspen-net daemon connected to cluster, SOCKS5 on :1080
3. **Guest A**: Launch CH microVM, wait for HTTP server on 10.10.1.2:8080
4. **Publish**: Register guest A's service as `my-svc` via CLI
5. **Guest B**: Launch CH microVM
6. **Route**: From guest B, curl through host SOCKS5 to `my-svc.aspen:8080`
7. **Verify**: Response content matches what guest A serves
8. **Cleanup**: Stop VMs, daemon, nodes
