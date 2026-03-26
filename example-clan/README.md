# Aspen Example Clan

3-node Aspen cluster on GMK mini PCs, managed with [Clan](https://clan.lol).

## Machines

| Machine | IP | Role | CPU | RAM | Disk |
|---------|----|------|-----|-----|------|
| gmk1 | 192.168.1.146 | Node 1 (initial leader) | Intel N150 (4c) | 16 GB | 512 GB NVMe |
| gmk2 | 192.168.1.114 | Node 2 (follower) | Intel N150 (4c) | 16 GB | 512 GB NVMe |
| gmk3 | 192.168.1.40 | Node 3 (follower) | Intel N150 (4c) | 16 GB | 512 GB NVMe |

## Quick Start

```bash
# Enter the dev shell (gets you the clan CLI)
cd example-clan
nix develop

# Generate vars (secrets, SSH keys)
clan vars generate

# Deploy to all machines
clan machines update gmk1
clan machines update gmk2
clan machines update gmk3
```

## Post-Deploy: Form the Cluster

After all three nodes are running:

```bash
# SSH to gmk1
ssh -i ~/.ssh/framework root@192.168.1.146

# Initialize the Raft cluster (node 1 as single-node, then add nodes 2 and 3)
aspen-cli --node 1 cluster init
aspen-cli --node 1 cluster add-learner 2
aspen-cli --node 1 cluster add-learner 3
aspen-cli --node 1 cluster change-membership 1 2 3

# Verify
aspen-cli --node 1 cluster metrics
aspen-cli --node 2 cluster metrics
aspen-cli --node 3 cluster metrics

# Test KV
aspen-cli --node 1 kv set hello world
aspen-cli --node 2 kv get hello  # should return "world"
aspen-cli --node 3 kv get hello  # should return "world"
```

## Connectivity

All machines are on the same LAN (192.168.1.0/24). Aspen nodes connect via
iroh QUIC (relay mode = default), so they discover each other through iroh's
relay infrastructure. No port forwarding needed beyond UDP 7777.

For the Raft cluster to form, nodes 2 and 3 need to know node 1's iroh
endpoint ID. The `aspen-cli cluster add-learner` command handles this — pass
the iroh endpoint ID that node 1 prints at startup.

## What's Deployed

Each node runs:

- **aspen-node**: Raft consensus + KV store + Forge + CI + blob storage
- **CI workers**: 2 local-executor workers per node
- **iroh**: P2P networking on UDP port 7777

## SSH Access

All machines accept the `framework` SSH key:

```bash
ssh -i ~/.ssh/framework root@192.168.1.146  # gmk1
ssh -i ~/.ssh/framework root@192.168.1.114  # gmk2
ssh -i ~/.ssh/framework root@192.168.1.40   # gmk3
```

## Files

```
example-clan/
├── flake.nix                         # Clan flake with aspen input
├── clan.nix                          # Inventory: machines, services, aspen config
├── machines/
│   ├── gmk1/
│   │   ├── configuration.nix         # NixOS config
│   │   └── hardware-configuration.nix
│   ├── gmk2/
│   │   ├── configuration.nix
│   │   └── hardware-configuration.nix
│   └── gmk3/
│       ├── configuration.nix
│       └── hardware-configuration.nix
└── README.md
```
