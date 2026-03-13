# aspen-testing-patchbay

Network simulation testing for Aspen using [patchbay](https://github.com/n0-computer/patchbay).

Spawns real Aspen nodes inside Linux network namespaces with configurable
NAT types, link conditions, and multi-region topologies. Tests exercise
real iroh QUIC connections through simulated network infrastructure.

## Requirements

- **Linux** with unprivileged user namespaces enabled
- **nft** (nftables) and **tc** (iproute2) in PATH
- No root privileges required

### Checking prerequisites

```bash
# Check userns support
sysctl kernel.unprivileged_userns_clone  # should be 1

# Ubuntu 24.04+ with AppArmor
sysctl kernel.apparmor_restrict_unprivileged_userns  # should be 0

# Enable if needed
sudo sysctl -w kernel.unprivileged_userns_clone=1
sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0
```

## Running tests

```bash
# Using nextest profile
cargo nextest run -P patchbay -p aspen-testing-patchbay

# Or directly
cargo test -p aspen-testing-patchbay
```

Tests automatically skip if prerequisites are not met (no failure).

## Test categories

### NAT tests (`patchbay_nat_tests.rs`)

- Public baseline (no NAT)
- Home NAT cluster formation
- Corporate NAT
- CGNAT (carrier-grade NAT)
- Mixed NAT topologies
- Leader failover across NAT boundaries

### Fault injection tests (`patchbay_fault_tests.rs`)

- Region partition (majority quorum, minority rejection, heal + catchup)
- Latency injection (sub-election-timeout, exceeding election timeout)
- Packet loss (10%, 50%)
- Link down/up (follower, leader failover)
- Dynamic NAT mode change

## Architecture

```
Test thread                    Patchbay namespace
    |                              |
    |-- NodeHandle (channels) ---->| bootstrap_node()
    |   .write_kv()                |   iroh::Endpoint (bound in namespace)
    |   .read_kv()                 |   Raft node (in-memory storage)
    |   .get_leader()              |   RaftRpcServer (accepts connections)
    |   .init_cluster()            |   command loop (processes NodeCommand)
    |                              |
```

Each node runs inside a patchbay device namespace via `device.spawn()`.
The iroh endpoint binds to the namespace's network stack, so all QUIC
traffic flows through the simulated topology (routers, NAT, link conditions).

Test code communicates with nodes via `tokio::sync::mpsc` channels,
which work across namespace boundaries since they're memory-based.
