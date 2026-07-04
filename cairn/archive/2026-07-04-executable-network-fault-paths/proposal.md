## Why

VM fault descriptors and receipts already model network faults, but the current VM path can record network partition support as unavailable when the image lacks network-control tooling. That unavailable boundary is correct, but it means the most important transport faults do not become executable evidence on capable hosts.

## What Changes

- Add an explicit network-control capability probe for the NixOS VM image and test-driver environment.
- Implement bounded executable network delay, drop, one-way partition, rejoin, and asymmetric latency paths when the probe passes.
- Preserve unavailable evidence when host or VM support is absent, and prevent unavailable cases from satisfying pass claims.
- Add positive executable-network fixtures and negative fixtures for missing cleanup, stale topology, log-only pass, unsupported host pass, and unrejoined partition.

## Impact

Network fault evidence becomes executable on capable hosts while remaining honest on unsupported hosts. Reviewers can distinguish a skipped network capability from a real partition/rejoin run without relying on VM logs.
