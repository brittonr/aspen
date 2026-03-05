## Context

We have two proven VM integration test patterns:

1. **microvm-raft-virtiofs**: 3-node Raft cluster → `aspen-cluster-virtiofs-server` → VirtioFS socket → CH microVM → nginx serves files from VirtioFS mount. Proves storage path.
2. **microvm-net-mesh**: 3-node Raft cluster → `aspen-net` daemon (SOCKS5 + TunnelAcceptor) → socat bridge → CH microVM HTTP server → curl through SOCKS5 mesh. Proves networking path.

Both share the same foundation (3-node Raft, Cloud Hypervisor, TAP networking) but have never been combined. The test needs nested KVM (QEMU → Cloud Hypervisor).

## Goals / Non-Goals

**Goals:**

- Prove VirtioFS and service mesh work simultaneously in a single microVM
- Guest A boots with VirtioFS mount (Raft KV → AspenFs → vhost-user), runs nginx serving content from that mount, and is published as a mesh service
- Guest B boots with TAP networking, curls Guest A through the SOCKS5 mesh, receives content that originated from the Raft KV store
- Complete data path: KV write → VirtioFS → nginx → TAP → socat → TunnelAcceptor → iroh QUIC → SOCKS5 → TAP → curl (Guest B)

**Non-Goals:**

- No new Rust code — purely a VM test combining existing capabilities
- No DNS inside guests (SOCKS5-hostname resolution from the host is sufficient)
- No mTLS or capability tokens (permissive mode)
- No multi-host (that's the other proposal)
- No guest-side aspen-net daemon (host-side SOCKS5 serves both guests)

## Decisions

### 1. Single VirtioFS server, two guests — one with VirtioFS + TAP, one with TAP only

Guest A gets both a VirtioFS share (for serving content) and TAP (for HTTP traffic). Guest B gets only TAP + curl. This mirrors a real deployment: compute VMs have filesystem access, client VMs just need network.

**Alternative**: Both guests mount VirtioFS. Rejected — adds complexity without proving anything new. The interesting composition is storage+networking, not two storage mounts.

### 2. Reuse existing daemon binaries, don't build new ones

We use `aspen-cluster-virtiofs-server` (from the microvm-raft-virtiofs test) and `aspen-net` daemon (from microvm-net-mesh) as-is. This proves the daemons compose without modification.

### 3. Two TAP interfaces, two subnets

Same pattern as microvm-net-mesh: `vm-svc` (10.10.1.0/24) for Guest A, `vm-client` (10.10.2.0/24) for Guest B. VirtioFS uses a separate vhost-user socket, not the TAP.

### 4. Write KV data first, then verify it appears in VirtioFS

The test writes `index.html` and `status.json` to the Raft KV store via `aspen-cli kv set`, then verifies they're visible through nginx on Guest A's VirtioFS mount, then verifies they're reachable through the mesh from Guest B.

## Architecture

```
┌─────────────────────────── QEMU Host ──────────────────────────────┐
│                                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                         │
│  │ aspen-1  │  │ aspen-2  │  │ aspen-3  │  Raft cluster           │
│  │ :7001    │  │ :7002    │  │ :7003    │  (inmemory)             │
│  └──────────┘  └──────────┘  └──────────┘                         │
│        ↕ iroh QUIC                                                  │
│  ┌─────────────────────┐  ┌───────────────────────────┐           │
│  │ aspen-net daemon    │  │ aspen-cluster-virtiofs    │           │
│  │ SOCKS5 :1080        │  │ /tmp/aspenfs.sock         │           │
│  │ TunnelAcceptor      │  │ VirtioFS vhost-user       │           │
│  └─────────┬───────────┘  └──────────┬────────────────┘           │
│            │                          │                             │
│  ┌─────────┼──────────────────────────┼─────────────┐             │
│  │  TAP    │           VirtioFS share │             │             │
│  │ ┌───────┴────────┐  ┌─────────────┴──────────┐  │             │
│  │ │ vm-svc TAP     │  │ Guest A (CH microVM)   │  │             │
│  │ │ 10.10.1.1/24   │  │ 10.10.1.2              │  │             │
│  │ └────────────────┘  │ VirtioFS → /var/www     │  │             │
│  │                     │ nginx :8080              │  │             │
│  │                     └─────────────────────────┘  │             │
│  │                                                   │             │
│  │ ┌────────────────┐  ┌─────────────────────────┐  │             │
│  │ │ vm-client TAP  │  │ Guest B (CH microVM)   │  │             │
│  │ │ 10.10.2.1/24   │  │ 10.10.2.2              │  │             │
│  │ └────────────────┘  │ curl → SOCKS5           │  │             │
│  │                     └─────────────────────────┘  │             │
│  └───────────────────────────────────────────────────┘             │
└─────────────────────────────────────────────────────────────────────┘
```

**Data flow (full path):**

1. `aspen-cli kv set /www/index.html "hello"` → Raft KV
2. AspenFs VirtioFS daemon reads from Raft KV → serves via vhost-user socket
3. Guest A mounts VirtioFS at `/var/www/aspen`, nginx serves from there
4. socat on host bridges `localhost:9080 → 10.10.1.2:8080`
5. `aspen-cli net publish web-svc --port 9080` registers in mesh
6. Guest B (or host) curls through SOCKS5: `curl --socks5-hostname 127.0.0.1:1080 http://web-svc.aspen:9080/index.html`
7. SOCKS5 resolves `web-svc` → iroh QUIC tunnel → TunnelAcceptor → socat → Guest A nginx → VirtioFS → Raft KV

## Risks / Trade-offs

- **[Boot time]** Guest A needs both VirtioFS daemon and network ready → may take 2-3 minutes. Mitigation: generous timeouts (180s), start VirtioFS daemon before launching VM.
- **[Memory]** Running 3 Raft nodes + VirtioFS daemon + net daemon + 2 CH microVMs on a single QEMU host. Mitigation: 8GB QEMU RAM, 512MB per guest, inmemory storage backend.
- **[VirtioFS socket race]** VirtioFS daemon must create the vhost-user socket before CH starts. Mitigation: `wait_until_succeeds("test -S /tmp/aspenfs.sock")` before launching the VM.
- **[Complexity]** This is the most complex single test — combines patterns from two existing tests. Mitigation: reuse proven patterns verbatim from both tests, don't innovate on the test infrastructure.
