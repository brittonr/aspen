## Context

The single-host `microvm-net-mesh` test proves SOCKS5 → iroh QUIC tunnel → TunnelAcceptor routing, but the iroh tunnel connects over loopback (127.0.0.1). In production, the tunnel crosses a real network. NixOS test driver supports multi-node tests natively — each `nodes.<name>` definition becomes a separate QEMU VM with its own networking. This gives us real IP-level separation between hosts.

Current multi-host tests in the repo (`multi-node-cluster`, `multi-node-blob`, `multi-node-coordination`) prove Raft consensus across QEMU hosts. This test adds microVMs inside those hosts, creating a 4-layer stack: QEMU host → Cloud Hypervisor guest → service mesh → Raft cluster.

## Goals / Non-Goals

**Goals:**

- Prove service mesh routing works across physically separate hosts (separate QEMU VMs with distinct IPs)
- iroh QUIC tunnel between Host B's SOCKS5 proxy and Host A's TunnelAcceptor traverses a real virtual network link
- 3-node Raft cluster with nodes on both hosts (consensus crosses hosts)
- Guest A on Host A publishes HTTP service; Guest B on Host B reaches it through cross-host mesh
- Complete data path: Guest B (Host B) → TAP → SOCKS5 (Host B) → iroh QUIC → Host A → TunnelAcceptor → socat → TAP → Guest A HTTP

**Non-Goals:**

- No new Rust code — VM integration test only
- No 3+ hosts (2 is sufficient to prove cross-host routing)
- No VirtioFS (that's the other proposal — keep this focused on networking)
- No automatic failover testing (single publish, single route)
- No host-to-host NAT traversal (QEMU virtual bridge provides direct L2 connectivity)

## Decisions

### 1. Two QEMU hosts, 3 Raft nodes (2+1 split)

Host A runs nodes 1+2, Host B runs node 3. This is the simplest split that proves cross-host consensus. Node 1 on Host A is the initial leader.

**Alternative**: 3 hosts with 1 node each. Rejected — adds a third QEMU VM without proving anything beyond what 2 hosts demonstrate. Memory budget is already tight.

### 2. Each host runs its own aspen-net daemon

Host A's daemon publishes Guest A's service and runs TunnelAcceptor. Host B's daemon runs SOCKS5 proxy. Both connect to the same Raft cluster for service registry. The tunnel goes from Host B's SOCKS5 through iroh QUIC to Host A's TunnelAcceptor.

**Alternative**: Single shared daemon. Rejected — defeats the purpose. We need separate daemons to prove the iroh tunnel crosses the network.

### 3. NixOS test driver multi-node networking

Use `nodes.hostA` and `nodes.hostB` in the NixOS test. The test driver automatically creates a virtual network (192.168.1.0/24 by default). Each host gets a distinct IP. Raft nodes use `--bind-port` on their host IP.

### 4. Nested KVM on both hosts

Both QEMU hosts need `-enable-kvm -cpu host` for Cloud Hypervisor. Each host has one CH microVM: Guest A (HTTP server) on Host A, Guest B (curl client) on Host B.

### 5. Cross-host ticket sharing

Host A bootstraps node 1, generates the cluster ticket. The test script passes the ticket to Host B for node 3 and the aspen-net daemon. NixOS test driver makes this easy — the Python test script controls all nodes.

## Architecture

```
┌──── QEMU Host A (192.168.1.1) ─────┐    ┌──── QEMU Host B (192.168.1.2) ─────┐
│                                      │    │                                      │
│  ┌──────────┐  ┌──────────┐         │    │  ┌──────────┐                        │
│  │ aspen-1  │  │ aspen-2  │         │    │  │ aspen-3  │                        │
│  │ :7001    │  │ :7002    │         │    │  │ :7001    │                        │
│  └──────────┘  └──────────┘         │    │  └──────────┘                        │
│        ↕ iroh QUIC                   │    │        ↕ iroh QUIC                   │
│  ┌────────────────────┐             │    │  ┌────────────────────┐              │
│  │ aspen-net daemon   │             │    │  │ aspen-net daemon   │              │
│  │ TunnelAcceptor     │◄════════════╋════╋══│ SOCKS5 :1080       │              │
│  └────────┬───────────┘  iroh QUIC  │    │  └────────────────────┘              │
│           │ socat                    │    │                                      │
│  ┌────────┴───────────┐             │    │  ┌────────────────────┐              │
│  │ Guest A (CH)       │             │    │  │ Guest B (CH)       │              │
│  │ 10.10.1.2          │             │    │  │ 10.10.2.2          │              │
│  │ HTTP :8080         │             │    │  │ curl → SOCKS5      │              │
│  └────────────────────┘             │    │  └────────────────────┘              │
└──────────────────────────────────────┘    └──────────────────────────────────────┘
         ↑ virtual bridge (QEMU)  ↑
         └────────────────────────┘
```

**Data flow (cross-host):**

1. Host A publishes Guest A's HTTP service as `my-svc` in Raft KV
2. Host B's SOCKS5 receives `curl --socks5-hostname ... http://my-svc.aspen:9080/`
3. SOCKS5 resolves `my-svc` from Raft registry → gets Host A's endpoint_id + port
4. SOCKS5 opens iroh QUIC connection to Host A's endpoint (crosses QEMU virtual bridge)
5. Host A's TunnelAcceptor receives connection, connects to `127.0.0.1:9080`
6. socat forwards to Guest A at `10.10.1.2:8080`
7. Response flows back: Guest A → socat → TunnelAcceptor → iroh QUIC → SOCKS5 → curl

## IP Address Plan

| Entity | IP | Network |
|--------|-----|---------|
| Host A | 192.168.1.1 | QEMU bridge |
| Host B | 192.168.1.2 | QEMU bridge |
| Guest A | 10.10.1.2 | TAP vm-svc on Host A |
| Host A TAP | 10.10.1.1 | TAP vm-svc on Host A |
| Guest B | 10.10.2.2 | TAP vm-client on Host B |
| Host B TAP | 10.10.2.1 | TAP vm-client on Host B |

## Risks / Trade-offs

- **[Memory pressure]** 2 QEMU hosts × (3 Raft nodes shared + aspen-net daemon + CH microVM) = ~12-16GB total. Mitigation: Host A gets 6GB (2 Raft nodes + VMs), Host B gets 4GB (1 Raft node + VM). Use inmemory backend.
- **[Cross-host iroh discovery]** iroh endpoints need to find each other across the QEMU bridge. Mitigation: Raft cluster ticket includes endpoint addresses; `--relay-mode disabled` + `--disable-mdns` + `--disable-gossip` forces direct connection over known IPs.
- **[Timing complexity]** Host A must bootstrap Raft, generate ticket, share it with Host B before Host B can start its node. NixOS test driver handles this via Python script sequencing — Host B `wait_until_succeeds` on a shared file or log condition.
- **[Nested KVM depth]** NixOS test driver QEMU → Cloud Hypervisor → guest. Both QEMU instances need KVM. The CI machine must support this. Mitigation: guard with `hasExternalRepos` or similar flag.
- **[Test duration]** Multi-host tests take longer (~10-15 min). Mitigation: skip in `quick` nextest profile, only run in `ci` profile or explicit `nix build`.
