## Context

`CloudHypervisorWorker::on_start()` currently awaits `VmPool::initialize()`. In the node setup path this is spawned, but the initialization still starts immediately during node bootstrap and can consume the runtime/host path before the dogfood health check completes. The latest live failures moved past TAP permissions and initial health, then exposed two VM-CI bridge boundaries: relay-disabled VM workers must not receive host-generated loopback/LAN/VPN addresses, and host setup must insert bridge ingress accepts into the NixOS firewall chain itself because an accept verdict in an earlier compatibility nftables base chain does not bypass a later NixOS drop chain.

## Goals / Non-Goals

**Goals:**
- Let the base node become healthy before VM-CI warmup/golden-snapshot work begins.
- Keep VM-CI warmup automatic after startup.
- Preserve clear logs for VM pool initialization failures.
- Ensure relay-disabled VM workers receive a guest-routable bootstrap ticket instead of host-local direct addresses.
- Ensure host VM-CI setup installs firewall ingress rules in the effective NixOS nftables chain for guest->host Iroh/QUIC UDP.

**Non-Goals:**
- Prove multi-node VM-CI scheduling.
- Replace the VM worker architecture.
- Hide VM-CI readiness failures after the node is healthy.
- Remove loopback-preferred tickets for same-host dogfood/operator clients.

## Decisions

### 1. Non-blocking VM-CI worker start

**Choice:** Change `CloudHypervisorWorker::on_start()` so it spawns pool initialization and maintenance in a background lifecycle task and returns immediately.

**Rationale:** Worker pool prewarm is an optimization/readiness boundary for VM jobs, not a prerequisite for serving node health.

**Alternative:** Increase dogfood health timeout. Rejected because it leaves base node health coupled to a fallible VM bootstrap path.

### 2. Start maintenance only after initial warmup attempt

**Choice:** The background task runs `pool.initialize().await`, logs success/failure, then starts periodic maintenance.

**Rationale:** This preserves automatic eventual VM readiness while keeping first-boot failures visible.

### 3. VM-scoped guest ticket address set

**Choice:** When `host_iroh_port` is known, rewrite the ticket written to each VM workspace so every bootstrap peer retains its endpoint id but has exactly one direct address: the host bridge socket address such as `10.200.0.1:<host_iroh_port>`.

**Rationale:** The host ticket can validly include loopback, LAN, VPN, and Tailscale addresses for same-host or operator clients. Inside the VM guest those addresses are either wrong or unreachable, and relay-disabled workers can time out before registering.

**Alternative:** Globally prefer the bridge/loopback address in all tickets. Rejected because same-host dogfood clients still need loopback-preferred tickets, and host/operator clients should not inherit guest-only address filtering.

### 4. NixOS firewall-chain bridge ingress

**Choice:** Advance the VM-CI host network marker to v3 and make `setup-ci-network` insert idempotent bridge ingress/forward accepts into `inet nixos-fw` when that NixOS firewall table exists, while keeping the compatibility `aspen-ci-filter` and NAT setup.

**Rationale:** nftables accepts in an earlier base chain do not prevent a later base chain from dropping the same packet. The observed boundary had bridge L3 reachability but guest->host Iroh/QUIC timeout, which is consistent with ICMP being allowed while UDP to the host node is dropped by the main NixOS firewall chain.

**Alternative:** Keep retrying dogfood or only preserve the compatibility accept chain. Rejected because stale v2 setup can look configured while still failing the actual guest->host UDP boundary.

## Risks / Trade-offs

**Jobs could be queued before VM workers register.** The CI wait path already polls pipeline state; if VM readiness remains broken, the failure should be classified later as worker readiness/provisioning instead of node-health failure.

**Background task lifetime.** Reuse the existing maintenance task handle so shutdown aborts either initial warmup or maintenance.

**Ticket scoping is context-specific.** Only the guest workspace ticket is filtered; host-side workspace/FUSE clients continue to use bridge injection that preserves existing direct addresses.
