# Aspen Example Clan

2-node Aspen cluster on physical hardware, managed with [Clan](https://clan.lol).

## Machines

| Machine | Role | CPU | RAM | Disk | iroh Node ID |
|---------|------|-----|-----|------|-------------|
| aspen1 | Node 1 (initial leader) | Ryzen AI MAX+ 395 (32c) | 128 GB | 3.6 TB NVMe | `59a51033...` |
| aspen2 | Node 2 (follower) | Ryzen AI MAX+ 395 (32c) | 64 GB | 1.8 TB NVMe | `511ce97e...` |

Both machines are reachable via `iroh-ssh` proxy (P2P QUIC, NAT-traversing).

## Quick Start

```bash
# Enter the dev shell (gets you the clan CLI)
cd example-clan
nix develop

# Generate vars (secrets, SSH keys)
clan vars generate

# Deploy to both machines
clan machines update aspen1
clan machines update aspen2
```

## Post-Deploy: Form the Cluster

After both nodes are running:

```bash
# SSH to aspen1
ssh iroh-aspen1

# Initialize the Raft cluster (node 1 as single-node, then add node 2)
aspen-cli --node 1 cluster init
aspen-cli --node 1 cluster add-learner 2
aspen-cli --node 1 cluster change-membership 1 2

# Verify
aspen-cli --node 1 cluster metrics
aspen-cli --node 2 cluster metrics

# Test KV
aspen-cli --node 1 kv set hello world
aspen-cli --node 2 kv get hello  # should return "world"
```

## Connectivity

The machines connect via iroh QUIC (relay mode = default), so they find each
other through iroh's relay infrastructure. No direct IP/port forwarding needed.

For the Raft cluster to form, node 2 needs to know node 1's iroh endpoint ID.
The `aspen-cli cluster add-learner` command handles this — pass the iroh
endpoint ID that node 1 prints at startup.

## What's Deployed

Each node runs:
- **aspen-node**: Raft consensus + KV store + Forge + CI + blob storage
- **CI workers**: 4 local-executor workers (no nested VMs)
- **iroh**: P2P networking on UDP port 7777

## Files

```
example-clan/
├── flake.nix                         # Clan flake with aspen input
├── clan.nix                          # Inventory: machines, services, aspen config
├── machines/
│   ├── aspen1/
│   │   ├── configuration.nix         # NixOS config (auto-imported by Clan)
│   │   └── hardware-configuration.nix
│   └── aspen2/
│       ├── configuration.nix
│       └── hardware-configuration.nix
└── README.md
```
